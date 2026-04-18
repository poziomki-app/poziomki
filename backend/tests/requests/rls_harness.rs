//! Reusable helpers for the RLS test suite.
//!
//! These helpers never depend on a specific policy being enabled — they
//! return raw catalog facts (`pg_class.relrowsecurity`, role
//! privileges, SD function `proconfig`, etc.). Tier-A/B/C policy PRs
//! build on top of them.
//!
//! The harness intentionally connects via the shared test pool (API role
//! via `API_DATABASE_URL`, falling back to `DATABASE_URL`) so what the
//! tests observe matches what production code sees at runtime.

use std::collections::{BTreeMap, BTreeSet};

use diesel::sql_types::{Bool, Nullable, Text};
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl, SimpleAsyncConnection};
use poziomki_backend::db::{self, DbViewer};

#[derive(diesel::deserialize::QueryableByName)]
struct BoolRow {
    #[diesel(sql_type = Bool)]
    value: bool,
}

#[derive(diesel::deserialize::QueryableByName)]
struct TextRow {
    #[diesel(sql_type = Text)]
    value: String,
}

#[derive(diesel::deserialize::QueryableByName)]
struct TextPair {
    #[diesel(sql_type = Text)]
    a: String,
    #[diesel(sql_type = Text)]
    b: String,
}

#[derive(diesel::deserialize::QueryableByName)]
struct NullableTextPair {
    #[diesel(sql_type = Text)]
    a: String,
    #[diesel(sql_type = Nullable<Text>)]
    b: Option<String>,
}

/// True iff the named `public.<table>` has `rowsecurity` enabled on the
/// table itself. Returns `false` for missing tables (so callers can assert
/// on a known set without setup races).
pub async fn table_rls_enabled(table: &str) -> bool {
    let mut conn = db::conn().await.expect("pool");
    let row: Option<BoolRow> = diesel::sql_query(
        "SELECT c.relrowsecurity AS value \
         FROM pg_class c \
         JOIN pg_namespace n ON n.oid = c.relnamespace \
         WHERE n.nspname = 'public' AND c.relname = $1",
    )
    .bind::<Text, _>(table)
    .get_result(&mut conn)
    .await
    .ok();
    row.is_some_and(|r| r.value)
}

/// True iff the named table also has `FORCE ROW LEVEL SECURITY` set, which
/// is required so the owner role doesn't bypass policies by default.
pub async fn table_force_rls(table: &str) -> bool {
    let mut conn = db::conn().await.expect("pool");
    let row: Option<BoolRow> = diesel::sql_query(
        "SELECT c.relforcerowsecurity AS value \
         FROM pg_class c \
         JOIN pg_namespace n ON n.oid = c.relnamespace \
         WHERE n.nspname = 'public' AND c.relname = $1",
    )
    .bind::<Text, _>(table)
    .get_result(&mut conn)
    .await
    .ok();
    row.is_some_and(|r| r.value)
}

/// Set of SQL privileges granted to `role` on `public.<table>`, normalised
/// to uppercase. Empty when the role has no grants on that table.
pub async fn role_privileges(role: &str, table: &str) -> BTreeSet<String> {
    let mut conn = db::conn().await.expect("pool");
    let rows: Vec<TextRow> = diesel::sql_query(
        "SELECT privilege_type AS value \
         FROM information_schema.role_table_grants \
         WHERE grantee = $1 AND table_schema = 'public' AND table_name = $2",
    )
    .bind::<Text, _>(role)
    .bind::<Text, _>(table)
    .load(&mut conn)
    .await
    .expect("grants query");
    rows.into_iter().map(|r| r.value).collect()
}

/// `(rolbypassrls, rolcanlogin)` for a named role. Returns `(false, false)`
/// for unknown roles. Used to assert `poziomki_api` is NOBYPASSRLS and
/// `poziomki_worker` is BYPASSRLS.
pub async fn role_flags(role: &str) -> (bool, bool) {
    let mut conn = db::conn().await.expect("pool");
    let row: Option<TextPair> = diesel::sql_query(
        "SELECT \
            CASE WHEN rolbypassrls THEN 'true' ELSE 'false' END AS a, \
            CASE WHEN rolcanlogin THEN 'true' ELSE 'false' END AS b \
         FROM pg_roles WHERE rolname = $1",
    )
    .bind::<Text, _>(role)
    .get_result(&mut conn)
    .await
    .ok();
    row.map(|r| (r.a == "true", r.b == "true"))
        .unwrap_or_default()
}

/// Map of every SECURITY DEFINER function name in schema `app` to its
/// `proconfig` string, which encodes `SET search_path = ...`. Tests use
/// this to assert every helper is hardened with `pg_catalog, pg_temp`.
pub async fn sd_function_configs() -> BTreeMap<String, Option<String>> {
    let mut conn = db::conn().await.expect("pool");
    let rows: Vec<NullableTextPair> = diesel::sql_query(
        "SELECT p.proname AS a, \
                array_to_string(p.proconfig, ',') AS b \
         FROM pg_proc p \
         JOIN pg_namespace n ON n.oid = p.pronamespace \
         WHERE n.nspname = 'app' AND p.prosecdef = true \
         ORDER BY p.proname",
    )
    .load(&mut conn)
    .await
    .expect("sd config query");
    rows.into_iter().map(|r| (r.a, r.b)).collect()
}

// ---------------------------------------------------------------------------
// API-role connection for RLS visibility tests.
//
// The shared test pool (`db::conn()`) resolves via `build_test_app_context`
// which falls back to `DATABASE_URL` when `API_DATABASE_URL` is unset. In CI
// that URL is the owner connection, which bypasses RLS by default — fine for
// the baseline privilege queries but wrong for visibility tests that must
// exercise the same permissions the API sees at runtime.
//
// `open_api_role_conn` opens a raw `AsyncPgConnection` using
// `TEST_API_DATABASE_URL`, which the CI workflow wires to the freshly
// bootstrapped `poziomki_api` role. Missing env var → panic with a clear
// pointer so local runs fail loudly instead of silently testing the wrong
// role. Existing migration_contract / db_viewer tests keep using the shared
// pool, which is intentional — they need write access beyond what the API
// role has (e.g. installing test-only triggers).
// ---------------------------------------------------------------------------

fn api_role_database_url() -> String {
    std::env::var("TEST_API_DATABASE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .expect(
            "TEST_API_DATABASE_URL must be set for RLS tests. In CI this is wired in \
             .github/workflows/rust.yml after the role-bootstrap step; locally, set it to \
             postgres://poziomki_api:<pwd>@localhost:5432/<db> matching the role created by \
             infra/ops/postgres/setup-roles.sql",
        )
}

/// Open a fresh `AsyncPgConnection` authenticated as `poziomki_api`. Each
/// call returns a new connection; tier tests should reuse a single conn
/// across their tx to minimise connection churn.
pub async fn open_api_role_conn() -> AsyncPgConnection {
    let url = api_role_database_url();
    AsyncPgConnection::establish(&url)
        .await
        .expect("open poziomki_api connection")
}

/// Run `f` inside a transaction that sets the viewer GUCs and runs as
/// `poziomki_api`. Mirrors `db::with_viewer_tx` but binds the connection
/// role explicitly so RLS visibility tests prove they exercise the same
/// permissions the running API sees.
pub async fn with_api_viewer_tx<T, F>(
    user_id: i32,
    is_review_stub: bool,
    f: F,
) -> Result<T, diesel::result::Error>
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
    let mut conn = open_api_role_conn().await;
    conn.transaction::<T, diesel::result::Error, _>(move |conn| {
        Box::pin(async move {
            // Reuse the production primitive so the same SET LOCAL shape
            // ends up in the session as what handlers emit at runtime.
            let viewer = DbViewer {
                user_id,
                is_review_stub,
            };
            db::set_viewer_context(conn, viewer).await?;
            f(conn).await
        })
    })
    .await
}

/// Simple variant of the above for queries that don't need the viewer
/// context (e.g. asserting `current_user`). Opens a raw API-role
/// connection and runs a single query.
pub async fn api_role_current_user() -> String {
    let mut conn = open_api_role_conn().await;
    let row: TextRow = diesel::sql_query("SELECT current_user::text AS value")
        .get_result(&mut conn)
        .await
        .expect("current_user query");
    row.value
}

/// Mirror of `api_role_current_user` but for GUCs — lets a test assert
/// that after entering a viewer tx the session's `app.user_id` matches
/// the viewer we handed in.
pub async fn api_role_current_user_raw(conn: &mut AsyncPgConnection) -> String {
    let row: TextRow = diesel::sql_query("SELECT current_user::text AS value")
        .get_result(conn)
        .await
        .expect("current_user query");
    row.value
}

// Keep SimpleAsyncConnection in the import set — it's used transitively
// by `AsyncConnection::transaction`, and clippy would otherwise warn on
// the bare import.
#[allow(dead_code)]
const fn _keep_simple_async_connection<C: SimpleAsyncConnection>(_: &C) {}
