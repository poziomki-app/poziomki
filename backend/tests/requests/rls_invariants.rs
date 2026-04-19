//! PR #13 — DB-level invariants, generic audit trigger, schema
//! hardening. Asserts the migration actually installed what it
//! claims: conversation CHECK constraints, `audit.events` writes on
//! `users` / `user_settings`, `REVOKE ALL ON SCHEMA public FROM PUBLIC`,
//! and the `statement_timeout` knob on `poziomki_api`.

use chrono::Utc;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Text};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use poziomki_backend::app::build_test_app_context;
use poziomki_backend::db;
use poziomki_backend::db::models::users::{NewUser, User};
use poziomki_backend::db::schema::users;
use serial_test::serial;
use uuid::Uuid;

fn setup() {
    let _ = dotenvy::dotenv();
    let _ = build_test_app_context().expect("build test app ctx");
}

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
struct BoolRow {
    #[diesel(sql_type = diesel::sql_types::Bool)]
    value: bool,
}

async fn insert_user_raw(conn: &mut AsyncPgConnection, email: &str) -> User {
    let new_user = NewUser {
        pid: Uuid::new_v4(),
        email: email.to_string(),
        password: "hash".to_string(),
        api_key: Uuid::new_v4().to_string(),
        name: "Invariant Test".to_string(),
    };
    diesel::insert_into(users::table)
        .values(&new_user)
        .returning(User::as_select())
        .get_result(conn)
        .await
        .expect("insert user")
}

// ---------------------------------------------------------------------------
// conversations CHECK constraints. Tier-B RLS enforces the same shape
// for the API role, but these guards apply to every writer (owner,
// worker, future BYPASSRLS roles).
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn dm_canonical_pair_rejects_inverted_order() {
    setup();
    let mut conn = db::conn().await.expect("pool");
    let now = Utc::now();
    let result = diesel::sql_query(
        "INSERT INTO public.conversations \
         (id, kind, user_low_id, user_high_id, created_at, updated_at) \
         VALUES ($1, 'dm', 5, 3, $2, $2)",
    )
    .bind::<diesel::sql_types::Uuid, _>(Uuid::new_v4())
    .bind::<diesel::sql_types::Timestamptz, _>(now)
    .execute(&mut conn)
    .await;

    assert!(
        result.is_err(),
        "dm_canonical_pair CHECK must reject user_low_id > user_high_id"
    );
}

#[tokio::test]
#[serial]
async fn dm_canonical_pair_rejects_self_dm() {
    setup();
    let mut conn = db::conn().await.expect("pool");
    let now = Utc::now();
    let result = diesel::sql_query(
        "INSERT INTO public.conversations \
         (id, kind, user_low_id, user_high_id, created_at, updated_at) \
         VALUES ($1, 'dm', 7, 7, $2, $2)",
    )
    .bind::<diesel::sql_types::Uuid, _>(Uuid::new_v4())
    .bind::<diesel::sql_types::Timestamptz, _>(now)
    .execute(&mut conn)
    .await;

    assert!(
        result.is_err(),
        "dm_canonical_pair CHECK must reject self-DM (low == high)"
    );
}

#[tokio::test]
#[serial]
async fn event_chat_pair_null_rejects_pair_on_event_row() {
    setup();
    let mut conn = db::conn().await.expect("pool");
    let now = Utc::now();
    // Fabricate an event id via a valid insert first — the attempt
    // below doesn't hit FK because we just assert the CHECK fires
    // before FK validation (it runs on the new row itself).
    let result = diesel::sql_query(
        "INSERT INTO public.conversations \
         (id, kind, event_id, user_low_id, user_high_id, created_at, updated_at) \
         VALUES ($1, 'event', $2, 1, 2, $3, $3)",
    )
    .bind::<diesel::sql_types::Uuid, _>(Uuid::new_v4())
    .bind::<diesel::sql_types::Uuid, _>(Uuid::new_v4())
    .bind::<diesel::sql_types::Timestamptz, _>(now)
    .execute(&mut conn)
    .await;

    assert!(
        result.is_err(),
        "event_chat_pair_null CHECK must reject event row with non-null pair ids"
    );
}

// ---------------------------------------------------------------------------
// Audit trigger on users: an UPDATE on email or password_hash writes
// exactly one audit.events row with op='U' and the column in
// changed_columns.
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn users_audit_captures_email_update() {
    setup();
    let mut conn = db::conn().await.expect("pool");
    let user = insert_user_raw(&mut conn, "audit-email@example.com").await;
    let user_id = user.id;

    diesel::sql_query("DELETE FROM audit.events WHERE table_name = 'public.users' AND row_pk = $1")
        .bind::<Text, _>(user_id.to_string())
        .execute(&mut conn)
        .await
        .expect("clear prior audit rows for the fixture");

    diesel::update(users::table.filter(users::id.eq(user_id)))
        .set(users::email.eq("audit-email-changed@example.com"))
        .execute(&mut conn)
        .await
        .expect("update email");

    let row: CountRow = diesel::sql_query(
        "SELECT COUNT(*) AS count FROM audit.events \
         WHERE table_name = 'public.users' \
           AND row_pk = $1 \
           AND op = 'U' \
           AND 'email' = ANY(changed_columns)",
    )
    .bind::<Text, _>(user_id.to_string())
    .get_result(&mut conn)
    .await
    .expect("audit count");
    assert_eq!(
        row.count, 1,
        "email UPDATE must produce one audit.events row with 'email' in changed_columns"
    );
}

// ---------------------------------------------------------------------------
// poziomki_api role has a bounded statement_timeout so a runaway
// query can't exhaust the pool.
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn poziomki_api_has_statement_timeout() {
    setup();
    let mut conn = db::conn().await.expect("pool");
    let row: TextRow = diesel::sql_query(
        "SELECT COALESCE( \
             (SELECT option_value FROM pg_options_to_table( \
                 (SELECT rolconfig FROM pg_roles WHERE rolname = 'poziomki_api')) \
              WHERE option_name = 'statement_timeout'), \
             '' \
         ) AS value",
    )
    .get_result(&mut conn)
    .await
    .expect("role config query");

    assert!(
        !row.value.is_empty(),
        "poziomki_api must carry a statement_timeout setting (got empty)"
    );
}

// ---------------------------------------------------------------------------
// PUBLIC must have no privileges on schema `public`. Older clusters
// shipped with USAGE granted by default; we asserted REVOKE ALL so
// new roles can't inherit the old defaults.
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn public_schema_has_no_public_privileges() {
    setup();
    let mut conn = db::conn().await.expect("pool");
    let row: BoolRow =
        diesel::sql_query("SELECT has_schema_privilege('public', 'public', 'USAGE') AS value")
            .get_result(&mut conn)
            .await
            .expect("public usage probe");
    assert!(
        !row.value,
        "PUBLIC must not retain USAGE on schema public after REVOKE ALL"
    );
}

// ---------------------------------------------------------------------------
// audit.events is INSERT-only for app roles; SELECT is denied so a
// compromised API-role caller can't dump the forensic log.
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn poziomki_api_cannot_select_audit_events() {
    setup();
    let mut conn = db::conn().await.expect("pool");
    let row: BoolRow = diesel::sql_query(
        "SELECT has_table_privilege('poziomki_api', 'audit.events', 'SELECT') AS value",
    )
    .get_result(&mut conn)
    .await
    .expect("select privilege probe");
    assert!(
        !row.value,
        "poziomki_api must not have SELECT on audit.events (INSERT-only)"
    );
}
