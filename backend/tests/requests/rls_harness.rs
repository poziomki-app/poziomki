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
use diesel_async::RunQueryDsl;
use poziomki_backend::db;

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
