//! Tier-C RLS visibility tests.
//!
//! Two parties: Alice (creator of event E1, owner of upload U1) and
//! Bob (attendee of E1, owner of upload U2). A third user Carol
//! exists but has no relationship to either — used to probe the
//! write surface.
//!
//! Covers:
//!   * `events`: bucket SELECT; only the creator can UPDATE / DELETE
//!   * `event_attendees`: bucket SELECT; only the attendee can mutate
//!     their own attendance
//!   * `uploads`: SELECT allows anon-owned rows AND same-bucket owner
//!     rows; only the owner can mutate
//!   * Identity-column triggers on all three tables
//!   * Anon sees zero rows on every Tier-C table

use chrono::{Duration, Utc};
use diesel::prelude::*;
use diesel::sql_types::BigInt;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use poziomki_backend::app::{build_test_app_context, reset_test_database};
use poziomki_backend::db;
use poziomki_backend::db::models::profiles::NewProfile;
use poziomki_backend::db::models::users::{NewUser, User};
use poziomki_backend::db::schema::{event_attendees, events, profiles, uploads, users};
use serial_test::serial;
use uuid::Uuid;

use super::rls_harness;

#[derive(diesel::deserialize::QueryableByName)]
struct CountRow {
    #[diesel(sql_type = BigInt)]
    count: i64,
}

struct Fx {
    alice: User,
    alice_profile: Uuid,
    bob: User,
    bob_profile: Uuid,
    carol: User,
    event_id: Uuid,
    alice_upload: Uuid,
    anon_upload: Uuid,
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

async fn setup_fixture() -> Fx {
    let _ = dotenvy::dotenv();
    let _ = build_test_app_context().expect("test app ctx");
    reset_test_database().await.expect("truncate");

    let alice = insert_user("tier-c-alice@example.com").await;
    let alice_profile = insert_profile(&alice, "Alice").await;
    let bob = insert_user("tier-c-bob@example.com").await;
    let bob_profile = insert_profile(&bob, "Bob").await;
    let carol = insert_user("tier-c-carol@example.com").await;
    let _ = insert_profile(&carol, "Carol").await;

    let event_id = Uuid::new_v4();
    let alice_upload = Uuid::new_v4();
    let anon_upload = Uuid::new_v4();
    {
        let mut conn = db::conn().await.expect("pool");
        let now = Utc::now();

        diesel::insert_into(events::table)
            .values((
                events::id.eq(event_id),
                events::title.eq("E1"),
                events::starts_at.eq(now + Duration::days(1)),
                events::creator_id.eq(alice_profile),
                events::created_at.eq(now),
                events::updated_at.eq(now),
                events::requires_approval.eq(false),
            ))
            .execute(&mut conn)
            .await
            .expect("seed event");

        for (attendee_profile, status) in [(alice_profile, "going"), (bob_profile, "going")] {
            diesel::insert_into(event_attendees::table)
                .values((
                    event_attendees::event_id.eq(event_id),
                    event_attendees::profile_id.eq(attendee_profile),
                    event_attendees::status.eq(status),
                ))
                .execute(&mut conn)
                .await
                .expect("seed attendee");
        }

        diesel::insert_into(uploads::table)
            .values((
                uploads::id.eq(alice_upload),
                uploads::filename.eq("alice.webp"),
                uploads::owner_id.eq(alice_profile),
                uploads::context.eq("avatar"),
                uploads::mime_type.eq("image/webp"),
            ))
            .execute(&mut conn)
            .await
            .expect("seed alice upload");

        diesel::insert_into(uploads::table)
            .values((
                uploads::id.eq(anon_upload),
                uploads::filename.eq("system.webp"),
                uploads::owner_id.eq::<Option<Uuid>>(None),
                uploads::context.eq("system"),
                uploads::mime_type.eq("image/webp"),
            ))
            .execute(&mut conn)
            .await
            .expect("seed anon upload");
    }

    Fx {
        alice,
        alice_profile,
        bob,
        bob_profile,
        carol,
        event_id,
        alice_upload,
        anon_upload,
    }
}

// ---------------------------------------------------------------------------
// events: bucket SELECT, creator-only writes.
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn events_same_bucket_viewers_see_event() {
    let fx = setup_fixture().await;

    let alice_count = rls_harness::with_api_viewer_tx(fx.alice.id, false, |conn| {
        async move { Ok(count_rows(conn, "events").await) }.scope_boxed()
    })
    .await
    .expect("alice tx");
    let carol_count = rls_harness::with_api_viewer_tx(fx.carol.id, false, |conn| {
        async move { Ok(count_rows(conn, "events").await) }.scope_boxed()
    })
    .await
    .expect("carol tx");

    assert_eq!(alice_count, 1, "creator sees own event");
    assert_eq!(
        carol_count, 1,
        "non-creator in same stub bucket sees the event (discoverable)"
    );
}

#[tokio::test]
#[serial]
async fn events_non_creator_cannot_delete() {
    let fx = setup_fixture().await;
    let event_id = fx.event_id;

    let affected = rls_harness::with_api_viewer_tx(fx.bob.id, false, move |conn| {
        async move {
            let sql = format!("DELETE FROM public.events WHERE id = '{event_id}'");
            Ok(execute_count(conn, &sql).await)
        }
        .scope_boxed()
    })
    .await
    .expect("bob tx");
    assert_eq!(
        affected, 0,
        "only the creator can DELETE; same-bucket non-creator sees the row but has no write policy match"
    );
}

#[tokio::test]
#[serial]
async fn events_non_creator_cannot_update() {
    let fx = setup_fixture().await;
    let event_id = fx.event_id;

    let affected = rls_harness::with_api_viewer_tx(fx.bob.id, false, move |conn| {
        async move {
            let sql =
                format!("UPDATE public.events SET title = 'hijacked' WHERE id = '{event_id}'");
            Ok(execute_count(conn, &sql).await)
        }
        .scope_boxed()
    })
    .await
    .expect("bob tx");
    assert_eq!(affected, 0, "only the creator can UPDATE");
}

#[tokio::test]
#[serial]
async fn events_creator_id_immutable() {
    let fx = setup_fixture().await;
    let event_id = fx.event_id;
    let bob_profile = fx.bob_profile;

    let result: Result<(), diesel::result::Error> =
        rls_harness::with_api_viewer_tx(fx.alice.id, false, move |conn| {
            async move {
                let sql = format!(
                    "UPDATE public.events SET creator_id = '{bob_profile}' WHERE id = '{event_id}'"
                );
                diesel::sql_query(sql).execute(conn).await?;
                Ok(())
            }
            .scope_boxed()
        })
        .await;

    assert!(
        result.is_err(),
        "events identity trigger must reject creator_id change"
    );
}

// ---------------------------------------------------------------------------
// event_attendees: bucket SELECT, own-profile writes.
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn event_attendees_viewer_sees_bucket_roster() {
    let fx = setup_fixture().await;

    let alice_count = rls_harness::with_api_viewer_tx(fx.alice.id, false, |conn| {
        async move { Ok(count_rows(conn, "event_attendees").await) }.scope_boxed()
    })
    .await
    .expect("alice tx");

    assert_eq!(alice_count, 2, "Alice sees both attendees (same bucket)");
}

#[tokio::test]
#[serial]
async fn event_attendees_non_creator_non_owner_cannot_mutate_peer() {
    let fx = setup_fixture().await;
    let event_id = fx.event_id;
    let bob_profile = fx.bob_profile;
    let carol_id = fx.carol.id;

    // Carol is not the event creator and not Bob — her UPDATE of
    // Bob's attendance must be rejected.
    let affected = rls_harness::with_api_viewer_tx(carol_id, false, move |conn| {
        async move {
            let sql = format!(
                "UPDATE public.event_attendees SET status = 'declined' \
                 WHERE event_id = '{event_id}' AND profile_id = '{bob_profile}'"
            );
            Ok(execute_count(conn, &sql).await)
        }
        .scope_boxed()
    })
    .await
    .expect("carol tx");
    assert_eq!(
        affected, 0,
        "non-creator non-owner cannot flip another attendee's status"
    );
}

#[tokio::test]
#[serial]
async fn event_attendees_creator_can_approve_pending_peer() {
    let fx = setup_fixture().await;
    let event_id = fx.event_id;
    let carol_id = fx.carol.id;
    let alice_id = fx.alice.id;

    // Seed: Carol is pending on Alice's event (requires_approval
    // irrelevant here — we're testing the RLS surface, not the
    // business logic).
    let carol_profile = {
        let mut conn = db::conn().await.expect("pool");
        let pid = profiles::table
            .filter(profiles::user_id.eq(carol_id))
            .select(profiles::id)
            .first::<Uuid>(&mut conn)
            .await
            .expect("carol profile");
        diesel::insert_into(event_attendees::table)
            .values((
                event_attendees::event_id.eq(event_id),
                event_attendees::profile_id.eq(pid),
                event_attendees::status.eq("pending"),
            ))
            .execute(&mut conn)
            .await
            .expect("seed carol pending");
        pid
    };

    // Alice (event creator) approves Carol — must succeed under the
    // viewer_owns_event branch.
    let affected = rls_harness::with_api_viewer_tx(alice_id, false, move |conn| {
        async move {
            let sql = format!(
                "UPDATE public.event_attendees SET status = 'going' \
                 WHERE event_id = '{event_id}' AND profile_id = '{carol_profile}'"
            );
            Ok(execute_count(conn, &sql).await)
        }
        .scope_boxed()
    })
    .await
    .expect("alice tx");
    assert_eq!(
        affected, 1,
        "event creator must be able to approve a pending attendee (handler-level approval flow)"
    );
}

// ---------------------------------------------------------------------------
// uploads: public anon-owned rows + bucket-owner rows visible; only
// the owner can mutate.
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn uploads_viewer_sees_anon_and_bucket_owned() {
    let fx = setup_fixture().await;

    let alice_count = rls_harness::with_api_viewer_tx(fx.alice.id, false, |conn| {
        async move { Ok(count_rows(conn, "uploads").await) }.scope_boxed()
    })
    .await
    .expect("alice tx");

    assert_eq!(
        alice_count, 2,
        "both Alice's own upload and the anon-owned row are visible"
    );
}

#[tokio::test]
#[serial]
async fn uploads_non_owner_cannot_delete_owned_row() {
    let fx = setup_fixture().await;
    let target = fx.alice_upload;

    let affected = rls_harness::with_api_viewer_tx(fx.bob.id, false, move |conn| {
        async move {
            let sql = format!("DELETE FROM public.uploads WHERE id = '{target}'");
            Ok(execute_count(conn, &sql).await)
        }
        .scope_boxed()
    })
    .await
    .expect("bob tx");
    assert_eq!(affected, 0, "only the owner can DELETE their upload");
}

#[tokio::test]
#[serial]
async fn uploads_owner_id_immutable() {
    let fx = setup_fixture().await;
    let target = fx.alice_upload;
    let bob_profile = fx.bob_profile;

    let result: Result<(), diesel::result::Error> =
        rls_harness::with_api_viewer_tx(fx.alice.id, false, move |conn| {
            async move {
                let sql = format!(
                    "UPDATE public.uploads SET owner_id = '{bob_profile}' WHERE id = '{target}'"
                );
                diesel::sql_query(sql).execute(conn).await?;
                Ok(())
            }
            .scope_boxed()
        })
        .await;

    assert!(
        result.is_err(),
        "uploads identity trigger must reject owner_id change"
    );
}

#[tokio::test]
#[serial]
async fn uploads_nobody_can_claim_anon_upload() {
    let fx = setup_fixture().await;
    let target = fx.anon_upload;
    let alice_profile = fx.alice_profile;

    let result: Result<(), diesel::result::Error> =
        rls_harness::with_api_viewer_tx(fx.alice.id, false, move |conn| {
            async move {
                let sql = format!(
                    "UPDATE public.uploads SET owner_id = '{alice_profile}' WHERE id = '{target}'"
                );
                diesel::sql_query(sql).execute(conn).await?;
                Ok(())
            }
            .scope_boxed()
        })
        .await;

    // Either the immutability trigger fires (if USING passes) or RLS
    // filters the row out (USING requires owner_id IN viewer_profile_ids,
    // and NULL is not). Both outcomes mean "anon upload stays anon".
    let mut conn = db::conn().await.expect("pool");
    let row: CountRow = diesel::sql_query(
        "SELECT COUNT(*) AS count FROM public.uploads WHERE id = $1 AND owner_id IS NULL",
    )
    .bind::<diesel::sql_types::Uuid, _>(target)
    .get_result(&mut conn)
    .await
    .expect("post-check");
    assert_eq!(
        row.count, 1,
        "anon upload must remain anonymous regardless of which path blocked the UPDATE"
    );
    let _ = result; // don't care if Err vs Ok(0 rows)
}

// ---------------------------------------------------------------------------
// Anon: every Tier-C table returns zero rows when no viewer is set.
// Uploads specifically is non-trivial because the SELECT policy
// accepts owner_id IS NULL — the current_user_id() > 0 guard on the
// helper still makes anon see nothing for owner_id IN bucket, but
// anon-owned rows would leak. This test pins the full behaviour.
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn anon_tx_sees_zero_events_and_attendees() {
    let _ = setup_fixture().await;

    for table in ["events", "event_attendees"] {
        let t = table.to_string();
        let visible = rls_harness::with_api_viewer_tx(0, false, move |conn| {
            async move { Ok(count_rows(conn, &t).await) }.scope_boxed()
        })
        .await
        .expect("anon tx");
        assert_eq!(visible, 0, "anon must see zero rows on public.{table}");
    }
}

#[tokio::test]
#[serial]
async fn anon_tx_sees_only_anon_uploads() {
    let _fx = setup_fixture().await;

    // Only the anon-owned row (owner_id IS NULL) is visible to anon —
    // the bucket branch requires current_user_id > 0.
    let visible = rls_harness::with_api_viewer_tx(0, false, |conn| {
        async move { Ok(count_rows(conn, "uploads").await) }.scope_boxed()
    })
    .await
    .expect("anon tx");
    assert_eq!(
        visible, 1,
        "anon sees only owner_id IS NULL uploads — bucket branch is gated"
    );
}
