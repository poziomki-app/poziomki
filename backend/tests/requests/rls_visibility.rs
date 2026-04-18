//! Scaffolding for visibility tests that Tier-A/B/C policy PRs will
//! extend. The helpers here let a test say "inside viewer tx for user X,
//! how many rows of table Y are visible?" without every test redoing
//! the transaction plumbing.
//!
//! These tests run against a **dedicated API-role connection** opened via
//! `rls_harness::open_api_role_conn` — explicitly `poziomki_api`, not the
//! shared test pool which falls back to the owner role in CI. Running as
//! owner would silently bypass RLS and let tier tests pass against a
//! role that doesn't match production. The first test below asserts the
//! connection identity so any regression in the wiring fails loudly
//! instead of producing false-green visibility results.

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

use super::rls_harness;

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

#[derive(diesel::deserialize::QueryableByName)]
struct IntegerRow {
    #[diesel(sql_type = Integer)]
    value: i32,
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

async fn insert_user(email: &str) -> User {
    // Inserts go through the shared pool (owner role) — the API role
    // doesn't always have INSERT on `users` under future policies, and
    // test setup legitimately needs to seed cross-user data. Tier tests
    // read via the API-role connection, which is where RLS bites.
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

/// Sanity: the dedicated RLS test connection actually authenticates as
/// `poziomki_api`. If this ever flips back to the owner role, all
/// downstream visibility tests would silently pass against a role that
/// bypasses RLS.
#[tokio::test]
#[serial]
async fn api_role_connection_authenticates_as_poziomki_api() {
    setup().await;
    let who = rls_harness::api_role_current_user().await;
    assert_eq!(
        who, "poziomki_api",
        "RLS harness must connect as poziomki_api (got {who:?}) — check \
         TEST_API_DATABASE_URL"
    );
}

/// Harness smoke test: with two users inserted, the viewer's tx sees
/// the expected GUC and can count both rows in `users`. Once Tier-A
/// policy `users USING (id = app.current_user_id())` lands, the count
/// drops to 1 and this test must be updated in the same PR.
#[tokio::test]
#[serial]
async fn viewer_tx_smoke_baseline() {
    setup().await;
    let alice = insert_user("rls-alice@example.com").await;
    let _bob = insert_user("rls-bob@example.com").await;

    let (current_user, guc_user_id, guc_role, visible_users) =
        rls_harness::with_api_viewer_tx(alice.id, false, |conn| {
            async move {
                let who = rls_harness::api_role_current_user_raw(conn).await;
                let uid: TextRow =
                    diesel::sql_query("SELECT current_setting('app.user_id', true) AS value")
                        .get_result(conn)
                        .await?;
                let role: TextRow =
                    diesel::sql_query("SELECT current_setting('app.role', true) AS value")
                        .get_result(conn)
                        .await?;
                let visible = count_rows(conn, "users").await;
                Ok((who, uid.value, role.value, visible))
            }
            .scope_boxed()
        })
        .await
        .expect("viewer tx");

    assert_eq!(
        current_user, "poziomki_api",
        "viewer tx must run as poziomki_api, not the owner"
    );
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

    let alice_count = rls_harness::with_api_viewer_tx(alice.id, false, |conn| {
        async move { Ok(count_rows(conn, "users").await) }.scope_boxed()
    })
    .await
    .expect("alice tx");
    let bob_count = rls_harness::with_api_viewer_tx(bob.id, false, |conn| {
        async move { Ok(count_rows(conn, "users").await) }.scope_boxed()
    })
    .await
    .expect("bob tx");

    assert_eq!(alice_count, 2);
    assert_eq!(bob_count, 2);
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

    let got: i32 = rls_harness::with_api_viewer_tx(alice.id, false, |conn| {
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
    .await
    .expect("cast tx");

    assert_eq!(got, alice.id);
}
