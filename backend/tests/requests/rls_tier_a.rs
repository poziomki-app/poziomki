//! Tier-A RLS visibility tests.
//!
//! Seeds two users (Alice + Bob), each with their own profile, and
//! asserts that a viewer-scoped tx as Alice can see only her own rows
//! across every Tier-A table. Mirrors the migration in
//! `2026-04-19-010000_rls_tier_a_policies`.
//!
//! Seed writes go through the shared pool (owner / superuser), so
//! FORCE ROW LEVEL SECURITY is bypassed for setup; the visibility
//! assertions open a fresh `poziomki_api` connection and run inside
//! `with_api_viewer_tx`, which is where policies actually bite.

use chrono::{Duration, Utc};
use diesel::prelude::*;
use diesel::sql_types::BigInt;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use poziomki_backend::app::{build_test_app_context, reset_test_database};
use poziomki_backend::db;
use poziomki_backend::db::models::profiles::NewProfile;
use poziomki_backend::db::models::users::{NewUser, User};
use poziomki_backend::db::schema::{
    event_interactions, events, profile_blocks, profile_bookmarks, profile_tags, profiles,
    push_subscriptions, recommendation_feedback, reports, sessions, tags, task_completions,
    user_audit_log, user_settings, users, xp_scans,
};
use serial_test::serial;
use uuid::Uuid;

use super::rls_harness;

#[derive(diesel::deserialize::QueryableByName)]
struct CountRow {
    #[diesel(sql_type = BigInt)]
    count: i64,
}

/// Per-party fixture: user id, profile id, plus anything else a test
/// might need (pid for `user_audit_log`).
struct Party {
    user: User,
    profile_id: Uuid,
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

async fn setup_two_parties() -> (Party, Party) {
    let _ = dotenvy::dotenv();
    let _ = build_test_app_context().expect("test app ctx");
    reset_test_database().await.expect("truncate");

    let alice_u = insert_user("tier-a-alice@example.com").await;
    let alice_pid = insert_profile(&alice_u, "Alice").await;
    let bob_u = insert_user("tier-a-bob@example.com").await;
    let bob_pid = insert_profile(&bob_u, "Bob").await;
    (
        Party {
            user: alice_u,
            profile_id: alice_pid,
        },
        Party {
            user: bob_u,
            profile_id: bob_pid,
        },
    )
}

// ---------------------------------------------------------------------------
// users: own row only, scoped by stub bucket
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn users_viewer_sees_only_own_row() {
    let (alice, _bob) = setup_two_parties().await;

    let visible = rls_harness::with_api_viewer_tx(alice.user.id, false, |conn| {
        async move { Ok(count_rows(conn, "users").await) }.scope_boxed()
    })
    .await
    .expect("alice tx");

    assert_eq!(visible, 1, "Tier-A users policy: viewer sees only own row");
}

// ---------------------------------------------------------------------------
// profiles: same-stub-bucket cross-visibility, own-row write
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn profiles_viewer_sees_same_stub_bucket() {
    let (alice, _bob) = setup_two_parties().await;

    // Alice + Bob are both non-stub — Alice's viewer tx should see both
    // profile rows (same bucket) even though profiles has RLS on.
    let visible = rls_harness::with_api_viewer_tx(alice.user.id, false, |conn| {
        async move { Ok(count_rows(conn, "profiles").await) }.scope_boxed()
    })
    .await
    .expect("alice tx");
    assert_eq!(
        visible, 2,
        "Tier-A profiles policy should allow same-bucket cross-profile reads"
    );
}

// ---------------------------------------------------------------------------
// sessions: own user_id only
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn sessions_viewer_sees_only_own_user() {
    let (alice, bob) = setup_two_parties().await;

    // Seed a session per user via owner pool (superuser bypasses FORCE).
    {
        let mut conn = db::conn().await.expect("pool");
        for u in [&alice.user, &bob.user] {
            diesel::insert_into(sessions::table)
                .values((
                    sessions::user_id.eq(u.id),
                    sessions::token.eq(format!("token-{}", u.id)),
                    sessions::expires_at.eq(Utc::now() + Duration::days(7)),
                ))
                .execute(&mut conn)
                .await
                .expect("seed session");
        }
    }

    let alice_count = rls_harness::with_api_viewer_tx(alice.user.id, false, |conn| {
        async move { Ok(count_rows(conn, "sessions").await) }.scope_boxed()
    })
    .await
    .expect("alice tx");
    assert_eq!(alice_count, 1);
}

// ---------------------------------------------------------------------------
// user_settings
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn user_settings_viewer_sees_only_own_row() {
    let (alice, bob) = setup_two_parties().await;

    {
        let mut conn = db::conn().await.expect("pool");
        for u in [&alice.user, &bob.user] {
            diesel::insert_into(user_settings::table)
                .values((user_settings::user_id.eq(u.id),))
                .execute(&mut conn)
                .await
                .expect("seed settings");
        }
    }

    let alice_count = rls_harness::with_api_viewer_tx(alice.user.id, false, |conn| {
        async move { Ok(count_rows(conn, "user_settings").await) }.scope_boxed()
    })
    .await
    .expect("alice tx");
    assert_eq!(alice_count, 1);
}

// ---------------------------------------------------------------------------
// user_audit_log: resolves user_pid via subquery in policy
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn user_audit_log_viewer_sees_only_own_pid() {
    let (alice, bob) = setup_two_parties().await;

    {
        let mut conn = db::conn().await.expect("pool");
        for u in [&alice.user, &bob.user] {
            diesel::insert_into(user_audit_log::table)
                .values((
                    user_audit_log::id.eq(Uuid::new_v4()),
                    user_audit_log::user_pid.eq(u.pid),
                    user_audit_log::action.eq("test_action"),
                ))
                .execute(&mut conn)
                .await
                .expect("seed audit");
        }
    }

    let alice_count = rls_harness::with_api_viewer_tx(alice.user.id, false, |conn| {
        async move { Ok(count_rows(conn, "user_audit_log").await) }.scope_boxed()
    })
    .await
    .expect("alice tx");
    assert_eq!(alice_count, 1);
}

// ---------------------------------------------------------------------------
// push_subscriptions
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn push_subscriptions_viewer_sees_only_own_row() {
    let (alice, bob) = setup_two_parties().await;

    {
        let mut conn = db::conn().await.expect("pool");
        for u in [&alice.user, &bob.user] {
            diesel::insert_into(push_subscriptions::table)
                .values((
                    push_subscriptions::user_id.eq(u.id),
                    push_subscriptions::ntfy_topic.eq(format!("topic-{}", u.id)),
                ))
                .execute(&mut conn)
                .await
                .expect("seed push_subscription");
        }
    }

    let alice_count = rls_harness::with_api_viewer_tx(alice.user.id, false, |conn| {
        async move { Ok(count_rows(conn, "push_subscriptions").await) }.scope_boxed()
    })
    .await
    .expect("alice tx");
    assert_eq!(alice_count, 1);
}

// ---------------------------------------------------------------------------
// xp_scans
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn xp_scans_viewer_sees_only_own_scans() {
    let (alice, bob) = setup_two_parties().await;

    {
        let mut conn = db::conn().await.expect("pool");
        diesel::insert_into(xp_scans::table)
            .values((
                xp_scans::scanner_id.eq(alice.profile_id),
                xp_scans::scanned_id.eq(bob.profile_id),
                xp_scans::day.eq(Utc::now().date_naive()),
            ))
            .execute(&mut conn)
            .await
            .expect("seed alice->bob scan");
        diesel::insert_into(xp_scans::table)
            .values((
                xp_scans::scanner_id.eq(bob.profile_id),
                xp_scans::scanned_id.eq(alice.profile_id),
                xp_scans::day.eq(Utc::now().date_naive()),
            ))
            .execute(&mut conn)
            .await
            .expect("seed bob->alice scan");
    }

    let alice_count = rls_harness::with_api_viewer_tx(alice.user.id, false, |conn| {
        async move { Ok(count_rows(conn, "xp_scans").await) }.scope_boxed()
    })
    .await
    .expect("alice tx");
    assert_eq!(
        alice_count, 1,
        "Alice only sees the scan she issued, not Bob's scan of her"
    );
}

// ---------------------------------------------------------------------------
// task_completions
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn task_completions_viewer_sees_only_own_row() {
    let (alice, bob) = setup_two_parties().await;

    {
        let mut conn = db::conn().await.expect("pool");
        for p in [alice.profile_id, bob.profile_id] {
            diesel::insert_into(task_completions::table)
                .values((
                    task_completions::profile_id.eq(p),
                    task_completions::task_id.eq("daily"),
                    task_completions::day.eq(Utc::now().date_naive()),
                ))
                .execute(&mut conn)
                .await
                .expect("seed task_completion");
        }
    }

    let alice_count = rls_harness::with_api_viewer_tx(alice.user.id, false, |conn| {
        async move { Ok(count_rows(conn, "task_completions").await) }.scope_boxed()
    })
    .await
    .expect("alice tx");
    assert_eq!(alice_count, 1);
}

// ---------------------------------------------------------------------------
// profile_bookmarks
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn profile_bookmarks_viewer_sees_only_own_bookmarks() {
    let (alice, bob) = setup_two_parties().await;

    {
        let mut conn = db::conn().await.expect("pool");
        diesel::insert_into(profile_bookmarks::table)
            .values((
                profile_bookmarks::profile_id.eq(alice.profile_id),
                profile_bookmarks::target_profile_id.eq(bob.profile_id),
            ))
            .execute(&mut conn)
            .await
            .expect("seed alice bookmark");
        diesel::insert_into(profile_bookmarks::table)
            .values((
                profile_bookmarks::profile_id.eq(bob.profile_id),
                profile_bookmarks::target_profile_id.eq(alice.profile_id),
            ))
            .execute(&mut conn)
            .await
            .expect("seed bob bookmark");
    }

    let alice_count = rls_harness::with_api_viewer_tx(alice.user.id, false, |conn| {
        async move { Ok(count_rows(conn, "profile_bookmarks").await) }.scope_boxed()
    })
    .await
    .expect("alice tx");
    assert_eq!(alice_count, 1);
}

// ---------------------------------------------------------------------------
// profile_blocks (policy allows both directions)
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn profile_blocks_viewer_sees_both_directions() {
    let (alice, bob) = setup_two_parties().await;

    {
        let mut conn = db::conn().await.expect("pool");
        // Alice blocked Bob, AND Bob blocked a fictional third party.
        diesel::insert_into(profile_blocks::table)
            .values((
                profile_blocks::blocker_id.eq(alice.profile_id),
                profile_blocks::blocked_id.eq(bob.profile_id),
            ))
            .execute(&mut conn)
            .await
            .expect("alice blocks bob");
        diesel::insert_into(profile_blocks::table)
            .values((
                profile_blocks::blocker_id.eq(bob.profile_id),
                profile_blocks::blocked_id.eq(alice.profile_id),
            ))
            .execute(&mut conn)
            .await
            .expect("bob blocks alice");
    }

    let alice_count = rls_harness::with_api_viewer_tx(alice.user.id, false, |conn| {
        async move { Ok(count_rows(conn, "profile_blocks").await) }.scope_boxed()
    })
    .await
    .expect("alice tx");
    assert_eq!(
        alice_count, 2,
        "profile_blocks policy lets the viewer see both directions \
         (own blocks AND blocks targeting them) — chat needs both"
    );
}

// ---------------------------------------------------------------------------
// recommendation_feedback
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn recommendation_feedback_viewer_sees_only_own_row() {
    let (alice, bob) = setup_two_parties().await;

    // Seed a fake event we can reference — recommendation_feedback FKs it.
    let event_id = Uuid::new_v4();
    {
        let mut conn = db::conn().await.expect("pool");
        let now = Utc::now();
        diesel::insert_into(events::table)
            .values((
                events::id.eq(event_id),
                events::title.eq("Test Event"),
                events::starts_at.eq(now + Duration::days(1)),
                events::creator_id.eq(alice.profile_id),
                events::created_at.eq(now),
                events::updated_at.eq(now),
                events::requires_approval.eq(false),
            ))
            .execute(&mut conn)
            .await
            .expect("seed event");

        for p in [alice.profile_id, bob.profile_id] {
            diesel::insert_into(recommendation_feedback::table)
                .values((
                    recommendation_feedback::profile_id.eq(p),
                    recommendation_feedback::event_id.eq(event_id),
                    recommendation_feedback::feedback.eq("more"),
                ))
                .execute(&mut conn)
                .await
                .expect("seed feedback");
        }
    }

    let alice_count = rls_harness::with_api_viewer_tx(alice.user.id, false, |conn| {
        async move { Ok(count_rows(conn, "recommendation_feedback").await) }.scope_boxed()
    })
    .await
    .expect("alice tx");
    assert_eq!(alice_count, 1);
}

// ---------------------------------------------------------------------------
// event_interactions
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn event_interactions_viewer_sees_only_own_row() {
    let (alice, bob) = setup_two_parties().await;

    let event_id = Uuid::new_v4();
    {
        let mut conn = db::conn().await.expect("pool");
        let now = Utc::now();
        diesel::insert_into(events::table)
            .values((
                events::id.eq(event_id),
                events::title.eq("Test Event"),
                events::starts_at.eq(now + Duration::days(1)),
                events::creator_id.eq(alice.profile_id),
                events::created_at.eq(now),
                events::updated_at.eq(now),
                events::requires_approval.eq(false),
            ))
            .execute(&mut conn)
            .await
            .expect("seed event");

        for p in [alice.profile_id, bob.profile_id] {
            diesel::insert_into(event_interactions::table)
                .values((
                    event_interactions::profile_id.eq(p),
                    event_interactions::event_id.eq(event_id),
                    event_interactions::kind.eq("saved"),
                ))
                .execute(&mut conn)
                .await
                .expect("seed interaction");
        }
    }

    let alice_count = rls_harness::with_api_viewer_tx(alice.user.id, false, |conn| {
        async move { Ok(count_rows(conn, "event_interactions").await) }.scope_boxed()
    })
    .await
    .expect("alice tx");
    assert_eq!(alice_count, 1);
}

// ---------------------------------------------------------------------------
// profile_tags (same-stub-bucket read)
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn profile_tags_viewer_sees_same_bucket() {
    let (alice, bob) = setup_two_parties().await;

    let tag_id = Uuid::new_v4();
    {
        let mut conn = db::conn().await.expect("pool");
        let now = Utc::now();
        diesel::insert_into(tags::table)
            .values((
                tags::id.eq(tag_id),
                tags::name.eq("Music"),
                tags::scope.eq("interest"),
                tags::created_at.eq(now),
                tags::updated_at.eq(now),
            ))
            .execute(&mut conn)
            .await
            .expect("seed tag");
        for p in [alice.profile_id, bob.profile_id] {
            diesel::insert_into(profile_tags::table)
                .values((
                    profile_tags::profile_id.eq(p),
                    profile_tags::tag_id.eq(tag_id),
                ))
                .execute(&mut conn)
                .await
                .expect("seed profile_tag");
        }
    }

    let alice_count = rls_harness::with_api_viewer_tx(alice.user.id, false, |conn| {
        async move { Ok(count_rows(conn, "profile_tags").await) }.scope_boxed()
    })
    .await
    .expect("alice tx");
    assert_eq!(
        alice_count, 2,
        "profile_tags policy allows reading other users' tags within \
         the same stub bucket (matching needs this)"
    );
}

// ---------------------------------------------------------------------------
// reports
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn reports_viewer_sees_only_own_reports() {
    let (alice, bob) = setup_two_parties().await;

    {
        let mut conn = db::conn().await.expect("pool");
        for p in [alice.profile_id, bob.profile_id] {
            diesel::insert_into(reports::table)
                .values((
                    reports::reporter_id.eq(p),
                    reports::target_type.eq("profile"),
                    reports::target_id.eq(Uuid::new_v4()),
                    reports::reason.eq("spam"),
                    reports::description.eq::<Option<String>>(None),
                ))
                .execute(&mut conn)
                .await
                .expect("seed report");
        }
    }

    let alice_count = rls_harness::with_api_viewer_tx(alice.user.id, false, |conn| {
        async move { Ok(count_rows(conn, "reports").await) }.scope_boxed()
    })
    .await
    .expect("alice tx");
    assert_eq!(alice_count, 1);
}

// ---------------------------------------------------------------------------
// Anon: no viewer context → policies evaluate to NULL → zero visible rows.
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn anon_tx_sees_zero_users() {
    let (_alice, _bob) = setup_two_parties().await;

    // `with_api_viewer_tx` with user_id = 0 emits app.user_id = '0',
    // which `app.current_user_id()` returns as NULLIF('0','')::int = 0.
    // No user has id = 0, so the viewer sees no rows.
    let visible = rls_harness::with_api_viewer_tx(0, false, |conn| {
        async move { Ok(count_rows(conn, "users").await) }.scope_boxed()
    })
    .await
    .expect("anon tx");
    assert_eq!(visible, 0, "anon viewer must not see any users");
}
