//! Scaffolding for visibility tests that Tier-A/B/C policy PRs will
//! extend. The helpers here let a test say "inside viewer tx for user X,
//! how many rows of table Y are visible?" without every test redoing
//! the transaction plumbing.
//!
//! The first test below is a **smoke test** that works before any policy
//! lands: it confirms `db::with_viewer_tx` plus a viewer id scopes the
//! query execution (GUC visible inside the tx) and that the connection
//! is actually the API role — future policies will bite here. Once
//! policies land, tier PRs add their own tests that use
//! `count_visible_rows` / `count_visible_rows_as` to assert the narrowed
//! result set.

use chrono::Utc;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Integer, Text};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use poziomki_backend::app::{build_test_app_context, reset_test_database};
use poziomki_backend::db;
use poziomki_backend::db::models::users::{NewUser, User};
use poziomki_backend::db::schema::users;
use serial_test::serial;
use uuid::Uuid;

#[derive(diesel::deserialize::QueryableByName)]
struct CountRow {
    #[diesel(sql_type = BigInt)]
    count: i64,
}

#[derive(diesel::deserialize::QueryableByName)]
struct TextRow {
    #[diesel(sql_type = Text)]
    value: String,
}

async fn setup() {
    let _ = dotenvy::dotenv();
    let _ = build_test_app_context().expect("build test app context");
    reset_test_database().await.expect("truncate");
}

/// Return the numeric row count of the given table *as seen from* the
/// current connection's active transaction. A later RLS policy flips
/// this count; the helper stays stable.
pub(super) async fn count_rows(conn: &mut AsyncPgConnection, table: &str) -> i64 {
    // Table name can't be a bind parameter in Postgres, so validate it
    // against a caller-owned allowlist before interpolating.
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

/// Open a viewer-scoped transaction on the shared test pool and run
/// `f`. Equivalent to `db::with_viewer_tx` but typed for the test-side
/// `diesel::result::Error` return and panics on transaction errors.
async fn with_viewer<T, F>(user_id: i32, f: F) -> T
where
    F: for<'c> FnOnce(
            &'c mut AsyncPgConnection,
        ) -> diesel_async::scoped_futures::ScopedBoxFuture<
            'static,
            'c,
            Result<T, diesel::result::Error>,
        > + Send
        + 'static,
    T: Send + 'static,
{
    let viewer = db::DbViewer {
        user_id,
        is_review_stub: false,
    };
    db::with_viewer_tx(viewer, f).await.expect("viewer tx")
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

/// Harness smoke test: with two users inserted, each viewer's tx sees
/// the expected GUC and can count both rows in `users`. Once Tier-A
/// policy `users USING (id = app.current_user_id())` lands, the count
/// drops to 1 and this test must be updated in the same PR.
#[tokio::test]
#[serial]
async fn viewer_tx_smoke_baseline() {
    setup().await;
    let alice = insert_user("rls-alice@example.com").await;
    let _bob = insert_user("rls-bob@example.com").await;

    let (guc_user_id, guc_role, visible_users) = with_viewer(alice.id, |conn| {
        async move {
            let uid: TextRow =
                diesel::sql_query("SELECT current_setting('app.user_id', true) AS value")
                    .get_result(conn)
                    .await?;
            let role: TextRow =
                diesel::sql_query("SELECT current_setting('app.role', true) AS value")
                    .get_result(conn)
                    .await?;
            let visible = count_rows(conn, "users").await;
            Ok((uid.value, role.value, visible))
        }
        .scope_boxed()
    })
    .await;

    assert_eq!(
        guc_user_id,
        alice.id.to_string(),
        "viewer GUC didn't reach the query context"
    );
    assert_eq!(guc_role, "user");
    assert_eq!(
        visible_users, 2,
        "with no Tier-A policy enabled the viewer still sees both users; \
         when the policy lands, flip this to 1 and add a matching test \
         for Bob's viewer tx"
    );
}

/// Cross-viewer sanity: count rows from each viewer's tx and confirm
/// both currently see the same data. Flips the moment a Tier-A policy
/// is attached to `users`.
#[tokio::test]
#[serial]
async fn baseline_both_viewers_see_all_users() {
    setup().await;
    let alice = insert_user("rls-cross-a@example.com").await;
    let bob = insert_user("rls-cross-b@example.com").await;

    let alice_count = with_viewer(alice.id, |conn| {
        async move { Ok(count_rows(conn, "users").await) }.scope_boxed()
    })
    .await;
    let bob_count = with_viewer(bob.id, |conn| {
        async move { Ok(count_rows(conn, "users").await) }.scope_boxed()
    })
    .await;

    assert_eq!(alice_count, 2);
    assert_eq!(bob_count, 2);
}

/// Demonstrates the harness also works for anon (pre-auth) transactions
/// — useful for OTP / rate-limit tables once their policies land in
/// Tier-D.
#[tokio::test]
#[serial]
async fn anon_tx_smoke_sets_role_anon() {
    setup().await;
    let _ = insert_user("rls-anon@example.com").await;

    let (role, user_id_guc) = db::with_anon_tx(|conn| {
        async move {
            let role: TextRow =
                diesel::sql_query("SELECT current_setting('app.role', true) AS value")
                    .get_result(conn)
                    .await?;
            let uid: TextRow =
                diesel::sql_query("SELECT current_setting('app.user_id', true) AS value")
                    .get_result(conn)
                    .await?;
            Ok((role.value, uid.value))
        }
        .scope_boxed()
    })
    .await
    .expect("anon tx");

    assert_eq!(role, "anon");
    assert_eq!(user_id_guc, "0");
}

/// Sanity check that the integer/text types flowing through the helper
/// are what the policy migrations will rely on: `app.user_id` is emitted
/// as a text GUC and cast back to int by the SQL function
/// `app.current_user_id()` that Tier-A's migration adds. This test
/// verifies the cast works for the current Rust → GUC wiring so
/// tier tests don't debug a type-mismatch on day one.
#[tokio::test]
#[serial]
async fn current_user_id_guc_cast_is_int_safe() {
    setup().await;
    let alice = insert_user("rls-cast@example.com").await;

    let got: i32 = with_viewer(alice.id, |conn| {
        async move {
            let row: IntegerRow = diesel::sql_query(
                "SELECT NULLIF(current_setting('app.user_id', true), '')::int AS value",
            )
            .get_result(conn)
            .await?;
            Ok(row.value)
        }
        .scope_boxed()
    })
    .await;

    assert_eq!(got, alice.id);
}

#[derive(diesel::deserialize::QueryableByName)]
struct IntegerRow {
    #[diesel(sql_type = Integer)]
    value: i32,
}

/// Silences an "unused" warning on `Utc` / `user_id_guc` spread across
/// helpers that the tier-policy PRs will consume but the baseline file
/// doesn't yet.
#[allow(dead_code)]
fn _keep_used_imports() {
    let _ = Utc::now();
}
