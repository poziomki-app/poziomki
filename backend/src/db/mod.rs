pub mod models;
pub mod schema;

use deadpool::managed::Timeouts;
use deadpool::Runtime;
use diesel::pg::PgConnection;
use diesel::Connection;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::AsyncPgConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::env;
use std::sync::OnceLock;
use std::time::Duration;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

pub fn run_migrations(database_url: &str) -> Result<(), String> {
    let mut conn = PgConnection::establish(database_url)
        .map_err(|e| format!("Migration connection failed: {e}"))?;
    conn.run_pending_migrations(MIGRATIONS)
        .map_err(|e| format!("Migration failed: {e}"))?;
    Ok(())
}

pub type DbPool = Pool<AsyncPgConnection>;
pub type DbConn = diesel_async::pooled_connection::deadpool::Object<AsyncPgConnection>;

static POOL: OnceLock<DbPool> = OnceLock::new();

#[derive(Debug, Clone, Copy)]
struct PoolSettings {
    max_size: usize,
    wait_timeout: Option<Duration>,
    create_timeout: Option<Duration>,
    recycle_timeout: Option<Duration>,
}

impl Default for PoolSettings {
    fn default() -> Self {
        Self {
            max_size: 10,
            wait_timeout: Some(Duration::from_secs(5)),
            create_timeout: Some(Duration::from_secs(5)),
            recycle_timeout: Some(Duration::from_secs(5)),
        }
    }
}

fn parse_env_usize(name: &str, default: usize) -> Result<usize, String> {
    match env::var(name) {
        Ok(raw) if !raw.trim().is_empty() => raw
            .parse::<usize>()
            .map_err(|e| format!("{name} must be a positive integer: {e}")),
        _ => Ok(default),
    }
}

fn parse_env_timeout_ms(name: &str, default: Option<Duration>) -> Result<Option<Duration>, String> {
    match env::var(name) {
        Ok(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                Ok(default)
            } else if trimmed == "0" {
                Ok(None)
            } else {
                let ms = trimmed
                    .parse::<u64>()
                    .map_err(|e| format!("{name} must be an integer milliseconds value: {e}"))?;
                Ok(Some(Duration::from_millis(ms)))
            }
        }
        Err(_) => Ok(default),
    }
}

fn pool_settings_from_env() -> Result<PoolSettings, String> {
    let defaults = PoolSettings::default();
    let max_size = parse_env_usize("DB_POOL_MAX_SIZE", defaults.max_size)?;
    if max_size == 0 {
        return Err(String::from("DB_POOL_MAX_SIZE must be >= 1"));
    }
    Ok(PoolSettings {
        max_size,
        wait_timeout: parse_env_timeout_ms("DB_POOL_WAIT_TIMEOUT_MS", defaults.wait_timeout)?,
        create_timeout: parse_env_timeout_ms("DB_POOL_CREATE_TIMEOUT_MS", defaults.create_timeout)?,
        recycle_timeout: parse_env_timeout_ms(
            "DB_POOL_RECYCLE_TIMEOUT_MS",
            defaults.recycle_timeout,
        )?,
    })
}

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
    let settings = pool_settings_from_env()?;
    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
    let mut builder = Pool::builder(manager).max_size(settings.max_size);
    if settings.wait_timeout.is_some()
        || settings.create_timeout.is_some()
        || settings.recycle_timeout.is_some()
    {
        builder = builder.runtime(Runtime::Tokio1).timeouts(Timeouts {
            wait: settings.wait_timeout,
            create: settings.create_timeout,
            recycle: settings.recycle_timeout,
        });
    }
    let pool = builder
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
#[tracing::instrument(skip_all)]
pub async fn conn() -> Result<DbConn, diesel_async::pooled_connection::deadpool::PoolError> {
    POOL.get()
        .ok_or(diesel_async::pooled_connection::deadpool::PoolError::NoRuntimeSpecified)?
        .get()
        .await
}
