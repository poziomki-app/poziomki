pub mod models;
pub mod schema;

use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::AsyncPgConnection;
use std::sync::OnceLock;

pub type DbPool = Pool<AsyncPgConnection>;
pub type DbConn = diesel_async::pooled_connection::deadpool::Object<AsyncPgConnection>;

static POOL: OnceLock<DbPool> = OnceLock::new();

/// Initialize the global Diesel connection pool.
///
/// Idempotent: if the pool is already initialised, this is a no-op.
///
/// # Errors
/// Returns an error string if pool construction fails.
pub fn init_pool(database_url: &str) -> Result<(), String> {
    if POOL.get().is_some() {
        return Ok(());
    }
    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
    let pool = Pool::builder(manager)
        .max_size(10)
        .build()
        .map_err(|e| format!("Diesel pool init failed: {e}"))?;
    // Another thread may have raced us — that's fine, ignore the error.
    let _ = POOL.set(pool);
    Ok(())
}

/// Obtain a connection from the global pool.
///
/// # Errors
/// Returns an error when the pool is not yet initialised or when obtaining a
/// connection from the pool fails.
pub async fn conn() -> Result<DbConn, diesel_async::pooled_connection::deadpool::PoolError> {
    POOL.get()
        .ok_or(diesel_async::pooled_connection::deadpool::PoolError::NoRuntimeSpecified)?
        .get()
        .await
}
