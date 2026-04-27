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
    conversation_members, conversations, event_attendees, events, message_reactions, messages,
    profiles, users,
};
use serial_test::serial;
use uuid::Uuid;

use super::rls_harness;

#[derive(diesel::deserialize::QueryableByName)]
struct CountRow {
    #[diesel(sql_type = BigInt)]
    count: i64,
}

#[derive(diesel::deserialize::QueryableByName)]
struct NullableIdRow {
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Uuid>)]
    id: Option<Uuid>,
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
        status_text: None,
        status_emoji: None,
        status_expires_at: None,
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
// conversation_members INSERT is the most dangerous lever — if any
// viewer can insert themselves into any conversation, the whole
// membership boundary collapses. These negative tests pin the
// tightened INSERT policy:
//   * outsiders cannot self-join a random conversation
//   * a DM participant cannot add a third party to their DM
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn conversation_members_outsider_cannot_self_join() {
    let fx = setup_fixture().await;
    let dm_id = fx.dm_id;
    let carol_id = fx.carol.id;

    let result: Result<(), diesel::result::Error> =
        rls_harness::with_api_viewer_tx(carol_id, false, move |conn| {
            async move {
                let now = Utc::now();
                diesel::insert_into(conversation_members::table)
                    .values(&NewConversationMember {
                        conversation_id: dm_id,
                        user_id: carol_id,
                        joined_at: now,
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
        "conversation_members_insert must reject self-join into a DM the viewer isn't party to"
    );
}

#[tokio::test]
#[serial]
async fn conversation_members_dm_cannot_inject_third_party() {
    let fx = setup_fixture().await;
    let dm_id = fx.dm_id;
    let alice_id = fx.alice.id;
    let carol_id = fx.carol.id;

    let result: Result<(), diesel::result::Error> =
        rls_harness::with_api_viewer_tx(alice_id, false, move |conn| {
            async move {
                let now = Utc::now();
                diesel::insert_into(conversation_members::table)
                    .values(&NewConversationMember {
                        conversation_id: dm_id,
                        user_id: carol_id,
                        joined_at: now,
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
        "conversation_members_insert DM branch must reject adding a user who isn't part of the DM pair"
    );
}

// ---------------------------------------------------------------------------
// PK-move trigger: a viewer UPDATEing their membership row to a
// different conversation_id must fail loudly (RLS WITH CHECK can't
// compare old vs new, so a BEFORE UPDATE trigger does the job).
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn conversation_members_pk_change_rejected() {
    let fx = setup_fixture().await;
    // Seed an unrelated conversation that Alice isn't a member of so
    // she has a real target to try moving into.
    let decoy_id = Uuid::new_v4();
    {
        let mut conn = db::conn().await.expect("pool");
        let now = Utc::now();
        diesel::insert_into(conversations::table)
            .values(&NewConversation {
                id: decoy_id,
                kind: "dm".into(),
                title: None,
                event_id: None,
                user_low_id: Some(fx.bob.id),
                user_high_id: Some(fx.carol.id),
                created_at: now,
                updated_at: now,
            })
            .execute(&mut conn)
            .await
            .expect("seed decoy");
    }

    let alice_id = fx.alice.id;
    let result: Result<(), diesel::result::Error> =
        rls_harness::with_api_viewer_tx(alice_id, false, move |conn| {
            async move {
                let sql = format!(
                    "UPDATE public.conversation_members SET conversation_id = '{decoy_id}' \
                     WHERE user_id = {alice_id}"
                );
                diesel::sql_query(sql).execute(conn).await?;
                Ok(())
            }
            .scope_boxed()
        })
        .await;

    assert!(
        result.is_err(),
        "conversation_members PK-move trigger must reject conversation_id UPDATE"
    );
}

// ---------------------------------------------------------------------------
// Low-finding fix: leaving a conversation must revoke edit/delete
// rights on past messages, not just SELECT visibility.
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn messages_update_rejected_after_leaving_conversation() {
    let fx = setup_fixture().await;
    let alice_id = fx.alice.id;
    let dm_id = fx.dm_id;
    let alice_msg = fx.alice_msg_id;

    // Alice leaves the DM via the owner pool (bypasses policies) so the
    // test isolates the post-leave edit attempt from the "can Alice
    // DELETE her own membership" policy question.
    {
        let mut conn = db::conn().await.expect("pool");
        diesel::delete(
            conversation_members::table
                .filter(conversation_members::conversation_id.eq(dm_id))
                .filter(conversation_members::user_id.eq(alice_id)),
        )
        .execute(&mut conn)
        .await
        .expect("remove alice membership");
    }

    let affected = rls_harness::with_api_viewer_tx(alice_id, false, move |conn| {
        async move {
            let sql =
                format!("UPDATE public.messages SET body = 'after-leave' WHERE id = '{alice_msg}'");
            Ok(execute_count(conn, &sql).await)
        }
        .scope_boxed()
    })
    .await
    .expect("alice tx");

    assert_eq!(
        affected, 0,
        "messages_update must require current membership, not just authorship"
    );
}

// ---------------------------------------------------------------------------
// Medium-finding fix: a viewer cannot update their own reaction onto
// a message they cannot see. Attack shape: Alice owns a reaction on
// Bob's message; there's also a hidden third-party message she
// happens to know the UUID of. The UPDATE must fail.
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn message_reactions_cannot_update_onto_hidden_message() {
    let fx = setup_fixture().await;
    let alice_id = fx.alice.id;
    let bob_msg = fx.bob_msg_id;

    // Seed a hidden conversation (Bob + Carol) + a message Alice can't see.
    let hidden_conv = Uuid::new_v4();
    let hidden_msg = Uuid::new_v4();
    let reaction_id = Uuid::new_v4();
    {
        let mut conn = db::conn().await.expect("pool");
        let now = Utc::now();
        diesel::insert_into(conversations::table)
            .values(&NewConversation {
                id: hidden_conv,
                kind: "dm".into(),
                title: None,
                event_id: None,
                user_low_id: Some(fx.bob.id),
                user_high_id: Some(fx.carol.id),
                created_at: now,
                updated_at: now,
            })
            .execute(&mut conn)
            .await
            .expect("seed hidden conv");
        for uid in [fx.bob.id, fx.carol.id] {
            diesel::insert_into(conversation_members::table)
                .values(&NewConversationMember {
                    conversation_id: hidden_conv,
                    user_id: uid,
                    joined_at: now,
                })
                .execute(&mut conn)
                .await
                .expect("seed hidden member");
        }
        diesel::insert_into(messages::table)
            .values(&NewMessage {
                id: hidden_msg,
                conversation_id: hidden_conv,
                sender_id: fx.bob.id,
                body: "hidden".into(),
                kind: "text".into(),
                reply_to_id: None,
                client_id: None,
                created_at: now,
            })
            .execute(&mut conn)
            .await
            .expect("seed hidden message");

        // Alice reacts to Bob's message in the DM she's in — fine.
        diesel::insert_into(message_reactions::table)
            .values((
                message_reactions::id.eq(reaction_id),
                message_reactions::message_id.eq(bob_msg),
                message_reactions::user_id.eq(alice_id),
                message_reactions::emoji.eq("👍"),
                message_reactions::created_at.eq(now),
            ))
            .execute(&mut conn)
            .await
            .expect("seed alice reaction");
    }

    let result: Result<(), diesel::result::Error> =
        rls_harness::with_api_viewer_tx(alice_id, false, move |conn| {
            async move {
                let sql = format!(
                    "UPDATE public.message_reactions SET message_id = '{hidden_msg}' \
                     WHERE id = '{reaction_id}'"
                );
                diesel::sql_query(sql).execute(conn).await?;
                Ok(())
            }
            .scope_boxed()
        })
        .await;

    // WITH CHECK failure surfaces as a diesel::result::Error::DatabaseError;
    // empty match + raw affected count both mean the row wasn't moved.
    if result.is_ok() {
        let mut conn = db::conn().await.expect("pool");
        let row: CountRow = diesel::sql_query(
            "SELECT COUNT(*) AS count FROM public.message_reactions \
             WHERE id = $1 AND message_id = $2",
        )
        .bind::<diesel::sql_types::Uuid, _>(reaction_id)
        .bind::<diesel::sql_types::Uuid, _>(hidden_msg)
        .get_result(&mut conn)
        .await
        .expect("post-check query");
        assert_eq!(
            row.count, 0,
            "message_reactions UPDATE must not move a reaction onto a hidden message"
        );
    }
}

// ---------------------------------------------------------------------------
// viewer_can_access_event status filter: pending attendees must NOT
// be able to create an event chat — the HTTP handler gates on
// `status = 'going'` and the RLS helper mirrors that.
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn event_conversation_insert_rejects_pending_attendee() {
    let fx = setup_fixture().await;
    let carol_id = fx.carol.id;

    // Carol has a `pending` attendance row on Bob's event. She's not
    // the creator, not confirmed, so event chat creation must fail.
    let event_id = Uuid::new_v4();
    let bob_profile: Uuid = {
        let mut conn = db::conn().await.expect("pool");
        let now = Utc::now();
        let bob_profile_id = profiles::table
            .filter(profiles::user_id.eq(fx.bob.id))
            .select(profiles::id)
            .first::<Uuid>(&mut conn)
            .await
            .expect("bob profile");
        let carol_profile_id = profiles::table
            .filter(profiles::user_id.eq(carol_id))
            .select(profiles::id)
            .first::<Uuid>(&mut conn)
            .await
            .expect("carol profile");
        diesel::insert_into(events::table)
            .values((
                events::id.eq(event_id),
                events::title.eq("Gathering"),
                events::starts_at.eq(now + chrono::Duration::days(1)),
                events::creator_id.eq(bob_profile_id),
                events::created_at.eq(now),
                events::updated_at.eq(now),
                events::requires_approval.eq(true),
            ))
            .execute(&mut conn)
            .await
            .expect("seed event");
        diesel::insert_into(event_attendees::table)
            .values((
                event_attendees::event_id.eq(event_id),
                event_attendees::profile_id.eq(carol_profile_id),
                event_attendees::status.eq("pending"),
            ))
            .execute(&mut conn)
            .await
            .expect("seed pending attendance");
        bob_profile_id
    };

    let _ = bob_profile;

    let result: Result<(), diesel::result::Error> =
        rls_harness::with_api_viewer_tx(carol_id, false, move |conn| {
            async move {
                let now = Utc::now();
                diesel::insert_into(conversations::table)
                    .values(&NewConversation {
                        id: Uuid::new_v4(),
                        kind: "event".into(),
                        title: Some("Gathering".into()),
                        event_id: Some(event_id),
                        user_low_id: None,
                        user_high_id: None,
                        created_at: now,
                        updated_at: now,
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
        "conversations_insert must reject event chat creation for a pending attendee"
    );
}

// ---------------------------------------------------------------------------
// API role must not be able to UPDATE or DELETE whole conversation
// rows. A naive member-only UPDATE/DELETE policy would let any chat
// participant nuke the room for everyone via ON DELETE CASCADE
// through conversation_members / messages / message_reactions.
// Legitimate event-chat cleanup routes through
// app.delete_event_and_chat() instead.
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn conversations_member_cannot_delete() {
    let fx = setup_fixture().await;
    let alice_id = fx.alice.id;
    let dm_id = fx.dm_id;

    let affected = rls_harness::with_api_viewer_tx(alice_id, false, move |conn| {
        async move {
            let sql = format!("DELETE FROM public.conversations WHERE id = '{dm_id}'");
            Ok(execute_count(conn, &sql).await)
        }
        .scope_boxed()
    })
    .await
    .expect("alice tx");
    assert_eq!(
        affected, 0,
        "no conversations_delete policy — member DELETE must be a no-op"
    );

    let mut conn = db::conn().await.expect("pool");
    let row: CountRow =
        diesel::sql_query("SELECT COUNT(*) AS count FROM public.conversations WHERE id = $1")
            .bind::<diesel::sql_types::Uuid, _>(dm_id)
            .get_result(&mut conn)
            .await
            .expect("post-check");
    assert_eq!(row.count, 1, "DM row must still exist");
}

#[tokio::test]
#[serial]
async fn conversations_member_cannot_update_metadata() {
    let fx = setup_fixture().await;
    let alice_id = fx.alice.id;
    let dm_id = fx.dm_id;

    let affected = rls_harness::with_api_viewer_tx(alice_id, false, move |conn| {
        async move {
            let sql =
                format!("UPDATE public.conversations SET title = 'tampered' WHERE id = '{dm_id}'");
            Ok(execute_count(conn, &sql).await)
        }
        .scope_boxed()
    })
    .await
    .expect("alice tx");
    assert_eq!(
        affected, 0,
        "no conversations_update policy — member UPDATE must be a no-op"
    );
}

// ---------------------------------------------------------------------------
// conversations identity-column trigger: a DM member cannot silently
// enrol a third party by swapping user_high_id, and cannot flip
// kind / event_id to switch which conversation_members_insert branch
// applies. These fields drive every downstream policy decision.
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn conversations_identity_change_rejected() {
    let fx = setup_fixture().await;
    let alice_id = fx.alice.id;
    let dm_id = fx.dm_id;
    let carol_id = fx.carol.id;

    // With no conversations_update policy, the member's UPDATE just
    // filters to zero rows — the identity trigger doesn't fire at all
    // from the API role path. We assert the mutation didn't land;
    // that's the property that matters. (The trigger still defends
    // owner / worker paths from accidental identity-column changes.)
    let affected = rls_harness::with_api_viewer_tx(alice_id, false, move |conn| {
        async move {
            let sql = format!(
                "UPDATE public.conversations SET user_high_id = {carol_id} \
                 WHERE id = '{dm_id}'"
            );
            Ok(execute_count(conn, &sql).await)
        }
        .scope_boxed()
    })
    .await
    .expect("alice tx");
    assert_eq!(
        affected, 0,
        "API role must not be able to reassign DM pair members"
    );

    let affected_kind = rls_harness::with_api_viewer_tx(alice_id, false, move |conn| {
        async move {
            let sql =
                format!("UPDATE public.conversations SET kind = 'event' WHERE id = '{dm_id}'");
            Ok(execute_count(conn, &sql).await)
        }
        .scope_boxed()
    })
    .await
    .expect("alice tx 2");
    assert_eq!(
        affected_kind, 0,
        "API role must not be able to flip conversation kind"
    );
}

// ---------------------------------------------------------------------------
// message_reactions PK-change trigger: a viewer cannot move their
// reaction to another message — even one they can see in a shared
// conversation. Defends against cross-conversation reaction injection
// where USING (old row visible) + WITH CHECK (new row visible) both
// pass but the semantics shift.
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn message_reactions_message_id_change_rejected() {
    let fx = setup_fixture().await;
    let alice_id = fx.alice.id;

    // Alice reacts to Bob's message in their shared DM — both
    // messages are visible to her, so the attack target is a valid
    // move under RLS predicates alone.
    let reaction_id = Uuid::new_v4();
    {
        let mut conn = db::conn().await.expect("pool");
        let now = Utc::now();
        diesel::insert_into(message_reactions::table)
            .values((
                message_reactions::id.eq(reaction_id),
                message_reactions::message_id.eq(fx.bob_msg_id),
                message_reactions::user_id.eq(alice_id),
                message_reactions::emoji.eq("🎉"),
                message_reactions::created_at.eq(now),
            ))
            .execute(&mut conn)
            .await
            .expect("seed reaction");
    }

    let target_msg = fx.alice_msg_id;
    let result: Result<(), diesel::result::Error> =
        rls_harness::with_api_viewer_tx(alice_id, false, move |conn| {
            async move {
                let sql = format!(
                    "UPDATE public.message_reactions SET message_id = '{target_msg}' \
                     WHERE id = '{reaction_id}'"
                );
                diesel::sql_query(sql).execute(conn).await?;
                Ok(())
            }
            .scope_boxed()
        })
        .await;

    assert!(
        result.is_err(),
        "message_reactions trigger must reject message_id UPDATE even to a visible message"
    );
}

// ---------------------------------------------------------------------------
// messages PK-change trigger: the sender must not be able to move
// their own message to another conversation they're also in. RLS can
// only evaluate the final row, so a `BEFORE UPDATE` trigger enforces
// conversation_id + sender_id immutability alongside the RLS policy.
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn messages_conversation_id_change_rejected() {
    let fx = setup_fixture().await;
    let alice_id = fx.alice.id;
    let alice_msg = fx.alice_msg_id;

    // Spin up a second DM (Alice + Carol) that Alice is a member of,
    // so her UPDATE has a legitimate target the WITH CHECK would
    // otherwise accept.
    let other_conv_id = Uuid::new_v4();
    {
        let mut conn = db::conn().await.expect("pool");
        let now = Utc::now();
        let pair: [i32; 2] = if alice_id < fx.carol.id {
            [alice_id, fx.carol.id]
        } else {
            [fx.carol.id, alice_id]
        };
        diesel::insert_into(conversations::table)
            .values(&NewConversation {
                id: other_conv_id,
                kind: "dm".into(),
                title: None,
                event_id: None,
                user_low_id: Some(pair[0]),
                user_high_id: Some(pair[1]),
                created_at: now,
                updated_at: now,
            })
            .execute(&mut conn)
            .await
            .expect("seed second dm");
        for uid in pair {
            diesel::insert_into(conversation_members::table)
                .values(&NewConversationMember {
                    conversation_id: other_conv_id,
                    user_id: uid,
                    joined_at: now,
                })
                .execute(&mut conn)
                .await
                .expect("seed second dm member");
        }
    }

    let result: Result<(), diesel::result::Error> =
        rls_harness::with_api_viewer_tx(alice_id, false, move |conn| {
            async move {
                let sql = format!(
                    "UPDATE public.messages SET conversation_id = '{other_conv_id}' \
                     WHERE id = '{alice_msg}'"
                );
                diesel::sql_query(sql).execute(conn).await?;
                Ok(())
            }
            .scope_boxed()
        })
        .await;

    assert!(
        result.is_err(),
        "messages trigger must reject cross-conversation UPDATE of conversation_id"
    );
}

// ---------------------------------------------------------------------------
// Positive event-chat join flow: a going attendee who is not yet a
// member of the event conversation can discover the existing row via
// the SD lookup helper AND self-insert their membership. This
// exercises the intersection of:
//   * find_event_conversation — must return the row even though the
//     viewer isn't yet a member (access gated on `viewer_can_access_event`)
//   * conversation_members_insert — must allow self-join for a going
//     attendee via the `conversation_meta_for_insert` SD helper
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn event_conversation_going_attendee_can_join_existing() {
    let fx = setup_fixture().await;
    let bob_id = fx.bob.id;
    let carol_id = fx.carol.id;

    // Bob owns the event; the conversation already exists with Bob as
    // the only member. Carol is a `going` attendee — she should be
    // able to find the conversation and self-insert a membership row.
    let event_id = Uuid::new_v4();
    let event_conv_id = Uuid::new_v4();
    {
        let mut conn = db::conn().await.expect("pool");
        let now = Utc::now();
        let bob_profile_id = profiles::table
            .filter(profiles::user_id.eq(bob_id))
            .select(profiles::id)
            .first::<Uuid>(&mut conn)
            .await
            .expect("bob profile");
        let carol_profile_id = profiles::table
            .filter(profiles::user_id.eq(carol_id))
            .select(profiles::id)
            .first::<Uuid>(&mut conn)
            .await
            .expect("carol profile");

        diesel::insert_into(events::table)
            .values((
                events::id.eq(event_id),
                events::title.eq("Join-Test"),
                events::starts_at.eq(now + chrono::Duration::days(1)),
                events::creator_id.eq(bob_profile_id),
                events::created_at.eq(now),
                events::updated_at.eq(now),
                events::requires_approval.eq(false),
            ))
            .execute(&mut conn)
            .await
            .expect("seed event");
        diesel::insert_into(event_attendees::table)
            .values((
                event_attendees::event_id.eq(event_id),
                event_attendees::profile_id.eq(carol_profile_id),
                event_attendees::status.eq("going"),
            ))
            .execute(&mut conn)
            .await
            .expect("seed carol attendance");

        diesel::insert_into(conversations::table)
            .values(&NewConversation {
                id: event_conv_id,
                kind: "event".into(),
                title: Some("Join-Test".into()),
                event_id: Some(event_id),
                user_low_id: None,
                user_high_id: None,
                created_at: now,
                updated_at: now,
            })
            .execute(&mut conn)
            .await
            .expect("seed event conv");
        diesel::insert_into(conversation_members::table)
            .values(&NewConversationMember {
                conversation_id: event_conv_id,
                user_id: bob_id,
                joined_at: now,
            })
            .execute(&mut conn)
            .await
            .expect("seed bob membership");
    }

    let lookup_id = rls_harness::with_api_viewer_tx(carol_id, false, move |conn| {
        async move {
            let row: NullableIdRow =
                diesel::sql_query("SELECT id FROM app.find_event_conversation($1) LIMIT 1")
                    .bind::<diesel::sql_types::Uuid, _>(event_id)
                    .get_result(conn)
                    .await
                    .unwrap_or(NullableIdRow { id: None });
            Ok(row.id)
        }
        .scope_boxed()
    })
    .await
    .expect("carol lookup tx");
    assert_eq!(
        lookup_id,
        Some(event_conv_id),
        "find_event_conversation must return the existing row for a going attendee"
    );

    // Self-insert must succeed.
    let insert_result: Result<(), diesel::result::Error> =
        rls_harness::with_api_viewer_tx(carol_id, false, move |conn| {
            async move {
                let now = Utc::now();
                diesel::insert_into(conversation_members::table)
                    .values(&NewConversationMember {
                        conversation_id: event_conv_id,
                        user_id: carol_id,
                        joined_at: now,
                    })
                    .execute(conn)
                    .await?;
                Ok(())
            }
            .scope_boxed()
        })
        .await;

    assert!(
        insert_result.is_ok(),
        "going attendee must be allowed to self-insert into the event conversation: {insert_result:?}"
    );

    // Confirm Carol is now visible as a member via the owner pool.
    let mut conn = db::conn().await.expect("pool");
    let row: CountRow = diesel::sql_query(
        "SELECT COUNT(*) AS count FROM public.conversation_members \
         WHERE conversation_id = $1 AND user_id = $2",
    )
    .bind::<diesel::sql_types::Uuid, _>(event_conv_id)
    .bind::<diesel::sql_types::Integer, _>(carol_id)
    .get_result(&mut conn)
    .await
    .expect("post-check");
    assert_eq!(row.count, 1, "Carol's membership row must be persisted");
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
