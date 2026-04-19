//! Tier-B RLS visibility tests.
//!
//! Three users: Alice + Bob share a DM, Carol is in no conversation.
//! Asserts that:
//!   * DM members see the conversation, each other's membership rows,
//!     and every message — while outsiders see nothing.
//!   * Only the sender can UPDATE / DELETE their own message even
//!     though the recipient can SELECT it.
//!   * Message reactions follow the parent message's visibility, and
//!     a user can only mutate their own reactions.
//!   * Anon viewers see zero chat rows across all four tables.

use chrono::Utc;
use diesel::prelude::*;
use diesel::sql_types::BigInt;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use poziomki_backend::app::{build_test_app_context, reset_test_database};
use poziomki_backend::db;
use poziomki_backend::db::models::conversation_members::NewConversationMember;
use poziomki_backend::db::models::conversations::NewConversation;
use poziomki_backend::db::models::messages::NewMessage;
use poziomki_backend::db::models::profiles::NewProfile;
use poziomki_backend::db::models::users::{NewUser, User};
use poziomki_backend::db::schema::{
    conversation_members, conversations, message_reactions, messages, profiles, users,
};
use serial_test::serial;
use uuid::Uuid;

use super::rls_harness;

#[derive(diesel::deserialize::QueryableByName)]
struct CountRow {
    #[diesel(sql_type = BigInt)]
    count: i64,
}

struct Fixture {
    alice: User,
    bob: User,
    carol: User,
    dm_id: Uuid,
    alice_msg_id: Uuid,
    bob_msg_id: Uuid,
}

async fn count_rows(conn: &mut AsyncPgConnection, table: &str) -> i64 {
    assert!(
        table.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
        "table name must be a simple identifier"
    );
    let row: CountRow = diesel::sql_query(format!("SELECT COUNT(*) AS count FROM {table}"))
        .get_result(conn)
        .await
        .expect("count");
    row.count
}

async fn execute_count(conn: &mut AsyncPgConnection, sql: &str) -> usize {
    diesel::sql_query(sql)
        .execute(conn)
        .await
        .expect("statement must succeed (RLS silently filters, not errors)")
}

async fn insert_user(email: &str) -> User {
    let mut conn = db::conn().await.expect("pool");
    let new_user = NewUser {
        pid: Uuid::new_v4(),
        email: email.to_string(),
        password: "hash".to_string(),
        api_key: Uuid::new_v4().to_string(),
        name: "Test".to_string(),
    };
    diesel::insert_into(users::table)
        .values(&new_user)
        .returning(User::as_select())
        .get_result(&mut conn)
        .await
        .expect("insert user")
}

async fn insert_profile(user: &User, name: &str) -> Uuid {
    let mut conn = db::conn().await.expect("pool");
    let now = Utc::now();
    let new_profile = NewProfile {
        id: Uuid::new_v4(),
        user_id: user.id,
        name: name.to_string(),
        bio: None,
        profile_picture: None,
        images: None,
        program: None,
        gradient_start: None,
        gradient_end: None,
        created_at: now,
        updated_at: now,
    };
    diesel::insert_into(profiles::table)
        .values(&new_profile)
        .returning(profiles::id)
        .get_result(&mut conn)
        .await
        .expect("insert profile")
}

async fn setup_fixture() -> Fixture {
    let _ = dotenvy::dotenv();
    let _ = build_test_app_context().expect("test app ctx");
    reset_test_database().await.expect("truncate");

    let alice = insert_user("tier-b-alice@example.com").await;
    let _ = insert_profile(&alice, "Alice").await;
    let bob = insert_user("tier-b-bob@example.com").await;
    let _ = insert_profile(&bob, "Bob").await;
    let carol = insert_user("tier-b-carol@example.com").await;
    let _ = insert_profile(&carol, "Carol").await;

    let (low, high) = if alice.id < bob.id {
        (alice.id, bob.id)
    } else {
        (bob.id, alice.id)
    };
    let dm_id = Uuid::new_v4();
    let alice_msg_id = Uuid::new_v4();
    let bob_msg_id = Uuid::new_v4();
    {
        let mut conn = db::conn().await.expect("pool");
        let now = Utc::now();
        diesel::insert_into(conversations::table)
            .values(&NewConversation {
                id: dm_id,
                kind: "dm".into(),
                title: None,
                event_id: None,
                user_low_id: Some(low),
                user_high_id: Some(high),
                created_at: now,
                updated_at: now,
            })
            .execute(&mut conn)
            .await
            .expect("seed dm");
        for uid in [alice.id, bob.id] {
            diesel::insert_into(conversation_members::table)
                .values(&NewConversationMember {
                    conversation_id: dm_id,
                    user_id: uid,
                    joined_at: now,
                })
                .execute(&mut conn)
                .await
                .expect("seed dm member");
        }
        for (id, sender) in [(alice_msg_id, alice.id), (bob_msg_id, bob.id)] {
            diesel::insert_into(messages::table)
                .values(&NewMessage {
                    id,
                    conversation_id: dm_id,
                    sender_id: sender,
                    body: format!("hello from {sender}"),
                    kind: "text".into(),
                    reply_to_id: None,
                    client_id: None,
                    created_at: now,
                })
                .execute(&mut conn)
                .await
                .expect("seed message");
        }
    }

    Fixture {
        alice,
        bob,
        carol,
        dm_id,
        alice_msg_id,
        bob_msg_id,
    }
}

// ---------------------------------------------------------------------------
// conversations: DM members see 1 row, outsiders see 0.
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn conversations_dm_members_see_only_their_dm() {
    let fx = setup_fixture().await;

    let alice_count = rls_harness::with_api_viewer_tx(fx.alice.id, false, |conn| {
        async move { Ok(count_rows(conn, "conversations").await) }.scope_boxed()
    })
    .await
    .expect("alice tx");
    let carol_count = rls_harness::with_api_viewer_tx(fx.carol.id, false, |conn| {
        async move { Ok(count_rows(conn, "conversations").await) }.scope_boxed()
    })
    .await
    .expect("carol tx");

    assert_eq!(alice_count, 1, "Alice is in the DM");
    assert_eq!(carol_count, 0, "Carol is in no conversation");
}

// ---------------------------------------------------------------------------
// conversation_members: DM members see both rows (self + peer),
// outsiders see none.
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn conversation_members_dm_members_see_both_rows() {
    let fx = setup_fixture().await;

    let alice_count = rls_harness::with_api_viewer_tx(fx.alice.id, false, |conn| {
        async move { Ok(count_rows(conn, "conversation_members").await) }.scope_boxed()
    })
    .await
    .expect("alice tx");
    let carol_count = rls_harness::with_api_viewer_tx(fx.carol.id, false, |conn| {
        async move { Ok(count_rows(conn, "conversation_members").await) }.scope_boxed()
    })
    .await
    .expect("carol tx");

    assert_eq!(
        alice_count, 2,
        "Alice sees her own + Bob's membership row (shared conversation)"
    );
    assert_eq!(carol_count, 0, "Carol is in no conversation");
}

// ---------------------------------------------------------------------------
// messages: members see every message, outsiders see none.
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn messages_members_see_both_messages() {
    let fx = setup_fixture().await;

    let alice_count = rls_harness::with_api_viewer_tx(fx.alice.id, false, |conn| {
        async move { Ok(count_rows(conn, "messages").await) }.scope_boxed()
    })
    .await
    .expect("alice tx");
    let bob_count = rls_harness::with_api_viewer_tx(fx.bob.id, false, |conn| {
        async move { Ok(count_rows(conn, "messages").await) }.scope_boxed()
    })
    .await
    .expect("bob tx");
    let carol_count = rls_harness::with_api_viewer_tx(fx.carol.id, false, |conn| {
        async move { Ok(count_rows(conn, "messages").await) }.scope_boxed()
    })
    .await
    .expect("carol tx");

    assert_eq!(alice_count, 2);
    assert_eq!(bob_count, 2);
    assert_eq!(carol_count, 0, "Carol must not see the DM's messages");
}

// ---------------------------------------------------------------------------
// messages UPDATE: only the sender can edit their own message even
// though the recipient can SELECT it.
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn messages_recipient_cannot_update_peers_message() {
    let fx = setup_fixture().await;
    let target = fx.bob_msg_id;

    let affected = rls_harness::with_api_viewer_tx(fx.alice.id, false, move |conn| {
        async move {
            let sql = format!("UPDATE public.messages SET body = 'edit' WHERE id = '{target}'");
            Ok(execute_count(conn, &sql).await)
        }
        .scope_boxed()
    })
    .await
    .expect("alice tx");
    assert_eq!(
        affected, 0,
        "messages_update must scope to sender only, not conversation membership"
    );
}

#[tokio::test]
#[serial]
async fn messages_recipient_cannot_delete_peers_message() {
    let fx = setup_fixture().await;
    let target = fx.bob_msg_id;

    let affected = rls_harness::with_api_viewer_tx(fx.alice.id, false, move |conn| {
        async move {
            let sql = format!("DELETE FROM public.messages WHERE id = '{target}'");
            Ok(execute_count(conn, &sql).await)
        }
        .scope_boxed()
    })
    .await
    .expect("alice tx");
    assert_eq!(affected, 0, "messages_delete must scope to sender only");
}

// ---------------------------------------------------------------------------
// message_reactions: visibility follows parent message; writes are
// user-scoped.
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn message_reactions_follow_message_visibility() {
    let fx = setup_fixture().await;

    // Seed: Alice reacts to Bob's message, Bob reacts to Alice's
    // message. Both reactions live inside the DM.
    {
        let mut conn = db::conn().await.expect("pool");
        let now = Utc::now();
        diesel::insert_into(message_reactions::table)
            .values((
                message_reactions::id.eq(Uuid::new_v4()),
                message_reactions::message_id.eq(fx.bob_msg_id),
                message_reactions::user_id.eq(fx.alice.id),
                message_reactions::emoji.eq("👍"),
                message_reactions::created_at.eq(now),
            ))
            .execute(&mut conn)
            .await
            .expect("alice reaction");
        diesel::insert_into(message_reactions::table)
            .values((
                message_reactions::id.eq(Uuid::new_v4()),
                message_reactions::message_id.eq(fx.alice_msg_id),
                message_reactions::user_id.eq(fx.bob.id),
                message_reactions::emoji.eq("🙏"),
                message_reactions::created_at.eq(now),
            ))
            .execute(&mut conn)
            .await
            .expect("bob reaction");
    }

    let alice_count = rls_harness::with_api_viewer_tx(fx.alice.id, false, |conn| {
        async move { Ok(count_rows(conn, "message_reactions").await) }.scope_boxed()
    })
    .await
    .expect("alice tx");
    let carol_count = rls_harness::with_api_viewer_tx(fx.carol.id, false, |conn| {
        async move { Ok(count_rows(conn, "message_reactions").await) }.scope_boxed()
    })
    .await
    .expect("carol tx");

    assert_eq!(
        alice_count, 2,
        "DM members see every reaction on the DM's messages"
    );
    assert_eq!(
        carol_count, 0,
        "Outsider sees no reactions even though the table has them"
    );
}

// ---------------------------------------------------------------------------
// Outsider cannot inject a message into a DM they aren't in.
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn messages_outsider_cannot_insert() {
    let fx = setup_fixture().await;
    let dm_id = fx.dm_id;
    let carol_id = fx.carol.id;

    let result: Result<(), diesel::result::Error> =
        rls_harness::with_api_viewer_tx(carol_id, false, move |conn| {
            async move {
                let now = Utc::now();
                diesel::insert_into(messages::table)
                    .values(&NewMessage {
                        id: Uuid::new_v4(),
                        conversation_id: dm_id,
                        sender_id: carol_id,
                        body: "intrusion".into(),
                        kind: "text".into(),
                        reply_to_id: None,
                        client_id: None,
                        created_at: now,
                    })
                    .execute(conn)
                    .await?;
                Ok(())
            }
            .scope_boxed()
        })
        .await;

    assert!(
        result.is_err(),
        "messages_insert WITH CHECK must reject cross-conversation sends"
    );
}

// ---------------------------------------------------------------------------
// Anon: every chat table returns zero rows.
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn anon_tx_sees_zero_chat_rows() {
    let _ = setup_fixture().await;

    for table in [
        "conversations",
        "conversation_members",
        "messages",
        "message_reactions",
    ] {
        let t = table.to_string();
        let visible = rls_harness::with_api_viewer_tx(0, false, move |conn| {
            async move { Ok(count_rows(conn, &t).await) }.scope_boxed()
        })
        .await
        .expect("anon tx");
        assert_eq!(visible, 0, "anon must see zero rows on public.{table}");
    }
}
