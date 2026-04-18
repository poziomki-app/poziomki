//! Viewer context for row-level security.
//!
//! Every per-user request sets a few session-local GUCs (`app.user_id`,
//! `app.is_stub`, `app.role`) that RLS policies consult. Because pgdog runs
//! in transaction-pool mode, the server backend is reattached after each
//! `COMMIT`; session-level `SET` would leak into the next request. All
//! helpers here use `SET LOCAL` inside an explicit transaction so the
//! context lives and dies with that transaction.
//!
//! `app.role` is descriptive only — the real trust boundary is the DB role
//! itself (`poziomki_api` vs `poziomki_worker`). A GUC can be overridden
//! from within any query, so it cannot gate privilege.

use diesel::deserialize::QueryableByName;
use diesel::sql_types::{Bool, Integer, Nullable, Text, Timestamptz, Uuid as SqlUuid, VarChar};
use diesel::OptionalExtension;
use diesel_async::scoped_futures::ScopedBoxFuture;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl, SimpleAsyncConnection};

/// Identity of the authenticated caller whose visibility scopes the current
/// transaction. Constructed from the session row after authentication.
#[derive(Clone, Copy, Debug)]
pub struct DbViewer {
    pub user_id: i32,
    pub is_review_stub: bool,
}

/// Apply the viewer as `SET LOCAL` GUCs.
///
/// Must run inside an explicit transaction. `with_viewer_tx` does that for
/// you; prefer this lower-level form only when an existing call site already
/// opens its own transaction (e.g. `build_transaction().serializable()`).
pub async fn set_viewer_context(
    conn: &mut AsyncPgConnection,
    viewer: DbViewer,
) -> Result<(), diesel::result::Error> {
    // Safe interpolation: user_id is i32, is_review_stub is bool.
    let sql = format!(
        "SET LOCAL app.user_id = '{}'; SET LOCAL app.is_stub = '{}'; SET LOCAL app.role = 'user'",
        viewer.user_id, viewer.is_review_stub
    );
    conn.batch_execute(&sql).await
}

/// Anonymous / pre-auth context. `app.user_id = 0`, `app.role = 'anon'`.
pub async fn set_anon_context(conn: &mut AsyncPgConnection) -> Result<(), diesel::result::Error> {
    conn.batch_execute(
        "SET LOCAL app.user_id = '0'; SET LOCAL app.is_stub = 'false'; SET LOCAL app.role = 'anon'",
    )
    .await
}

/// Run `f` inside a transaction that has `set_viewer_context(viewer)` as its
/// first statement. Use for the common case where the handler doesn't need
/// custom isolation.
pub async fn with_viewer_tx<'a, F, T>(viewer: DbViewer, f: F) -> Result<T, diesel::result::Error>
where
    F: for<'c> FnOnce(
            &'c mut AsyncPgConnection,
        ) -> ScopedBoxFuture<'a, 'c, Result<T, diesel::result::Error>>
        + Send
        + 'a,
    T: Send + 'a,
{
    let mut conn = crate::db::conn()
        .await
        .map_err(|_| diesel::result::Error::BrokenTransactionManager)?;
    conn.transaction::<T, diesel::result::Error, _>(move |conn| {
        Box::pin(async move {
            set_viewer_context(conn, viewer).await?;
            f(conn).await
        })
    })
    .await
}

/// `with_viewer_tx` but for anonymous / pre-auth endpoints.
pub async fn with_anon_tx<'a, F, T>(f: F) -> Result<T, diesel::result::Error>
where
    F: for<'c> FnOnce(
            &'c mut AsyncPgConnection,
        ) -> ScopedBoxFuture<'a, 'c, Result<T, diesel::result::Error>>
        + Send
        + 'a,
    T: Send + 'a,
{
    let mut conn = crate::db::conn()
        .await
        .map_err(|_| diesel::result::Error::BrokenTransactionManager)?;
    conn.transaction::<T, diesel::result::Error, _>(move |conn| {
        Box::pin(async move {
            set_anon_context(conn).await?;
            f(conn).await
        })
    })
    .await
}

// ---------------------------------------------------------------------------
// SECURITY DEFINER lookups
//
// Authentication queries `users` by email and `sessions` by hashed token
// before any viewer context exists. Once Tier-A policies are enabled, those
// plain SELECTs will return zero rows. The `app.*` functions defined in the
// auth_security_definer migration run as the owner and are tightly scoped to
// exact-match inputs, so the API role can authenticate without being granted
// broad read on the underlying tables.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, QueryableByName)]
pub struct AuthUserRow {
    #[diesel(sql_type = Integer)]
    pub id: i32,
    #[diesel(sql_type = SqlUuid)]
    pub pid: uuid::Uuid,
    #[diesel(sql_type = VarChar)]
    pub name: String,
    #[diesel(sql_type = VarChar)]
    pub email: String,
    #[diesel(sql_type = VarChar)]
    pub password: String,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub email_verified_at: Option<chrono::DateTime<chrono::Utc>>,
    #[diesel(sql_type = Bool)]
    pub is_review_stub: bool,
}

/// Exact-match user lookup by email. Bypasses RLS via SECURITY DEFINER.
pub async fn find_user_for_login(
    conn: &mut AsyncPgConnection,
    email: &str,
) -> Result<Option<AuthUserRow>, diesel::result::Error> {
    diesel::sql_query("SELECT * FROM app.find_user_for_login($1)")
        .bind::<Text, _>(email)
        .get_result::<AuthUserRow>(conn)
        .await
        .optional()
}

#[derive(Debug, Clone, QueryableByName)]
pub struct AuthSessionRow {
    #[diesel(sql_type = SqlUuid)]
    pub session_id: uuid::Uuid,
    #[diesel(sql_type = Integer)]
    pub user_id: i32,
    #[diesel(sql_type = SqlUuid)]
    pub user_pid: uuid::Uuid,
    #[diesel(sql_type = VarChar)]
    pub token: String,
    #[diesel(sql_type = Nullable<VarChar>)]
    pub ip_address: Option<String>,
    #[diesel(sql_type = Nullable<VarChar>)]
    pub user_agent: Option<String>,
    #[diesel(sql_type = Timestamptz)]
    pub expires_at: chrono::DateTime<chrono::Utc>,
    #[diesel(sql_type = Timestamptz)]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[diesel(sql_type = Timestamptz)]
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[diesel(sql_type = Bool)]
    pub is_review_stub: bool,
}

/// Exact-match session lookup by hashed token. Bypasses RLS via SECURITY DEFINER.
pub async fn resolve_session(
    conn: &mut AsyncPgConnection,
    token_hash: &str,
) -> Result<Option<AuthSessionRow>, diesel::result::Error> {
    diesel::sql_query("SELECT * FROM app.resolve_session($1)")
        .bind::<Text, _>(token_hash)
        .get_result::<AuthSessionRow>(conn)
        .await
        .optional()
}

// ---------------------------------------------------------------------------
// SECURITY DEFINER writes for the pre-auth users flows.
//
// Sign-up, email verification, and password reset all mutate `users` before
// any viewer context exists. The matching `app.*` functions are scoped to
// exactly those flows so we don't need to grant the API role broad
// INSERT/UPDATE on the underlying tables.
// ---------------------------------------------------------------------------

/// Insert a user row for the sign-up flow. Runs as owner so no viewer
/// context is required. Returns the same shape as `find_user_for_login`.
pub async fn create_user_for_signup(
    conn: &mut AsyncPgConnection,
    pid: uuid::Uuid,
    email: &str,
    password_hash: &str,
    api_key: &str,
    name: &str,
) -> Result<AuthUserRow, diesel::result::Error> {
    diesel::sql_query(
        "SELECT id, pid, name, email, password, email_verified_at, is_review_stub \
         FROM app.create_user_for_signup($1, $2, $3, $4, $5)",
    )
    .bind::<SqlUuid, _>(pid)
    .bind::<Text, _>(email)
    .bind::<Text, _>(password_hash)
    .bind::<Text, _>(api_key)
    .bind::<Text, _>(name)
    .get_result::<AuthUserRow>(conn)
    .await
}

/// Mark a user's email as verified. Idempotent.
pub async fn mark_email_verified(
    conn: &mut AsyncPgConnection,
    user_id: i32,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<(), diesel::result::Error> {
    diesel::sql_query("SELECT app.mark_email_verified($1, $2)")
        .bind::<Integer, _>(user_id)
        .bind::<Timestamptz, _>(now)
        .execute(conn)
        .await?;
    Ok(())
}

/// Record a hashed password-reset token on a user.
pub async fn set_password_reset_token(
    conn: &mut AsyncPgConnection,
    user_id: i32,
    token_hash: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<(), diesel::result::Error> {
    diesel::sql_query("SELECT app.set_password_reset_token($1, $2, $3)")
        .bind::<Integer, _>(user_id)
        .bind::<Text, _>(token_hash)
        .bind::<Timestamptz, _>(now)
        .execute(conn)
        .await?;
    Ok(())
}

#[derive(Debug, Clone, QueryableByName)]
pub struct PasswordResetUserRow {
    #[diesel(sql_type = Integer)]
    pub id: i32,
    #[diesel(sql_type = SqlUuid)]
    pub pid: uuid::Uuid,
    #[diesel(sql_type = VarChar)]
    pub email: String,
    #[diesel(sql_type = VarChar)]
    pub name: String,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub email_verified_at: Option<chrono::DateTime<chrono::Utc>>,
    #[diesel(sql_type = Bool)]
    pub is_review_stub: bool,
}

/// Look up a user by email + hashed reset token + TTL cutoff.
pub async fn find_user_for_password_reset(
    conn: &mut AsyncPgConnection,
    email: &str,
    token_hash: &str,
    cutoff: chrono::DateTime<chrono::Utc>,
) -> Result<Option<PasswordResetUserRow>, diesel::result::Error> {
    diesel::sql_query("SELECT * FROM app.find_user_for_password_reset($1, $2, $3)")
        .bind::<Text, _>(email)
        .bind::<Text, _>(token_hash)
        .bind::<Timestamptz, _>(cutoff)
        .get_result::<PasswordResetUserRow>(conn)
        .await
        .optional()
}

/// Rotate password, clear reset token, and invalidate sessions for a user.
pub async fn complete_password_reset(
    conn: &mut AsyncPgConnection,
    user_id: i32,
    new_password_hash: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<(), diesel::result::Error> {
    diesel::sql_query("SELECT app.complete_password_reset($1, $2, $3)")
        .bind::<Integer, _>(user_id)
        .bind::<Text, _>(new_password_hash)
        .bind::<Timestamptz, _>(now)
        .execute(conn)
        .await?;
    Ok(())
}
