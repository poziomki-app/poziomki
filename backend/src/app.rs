#[derive(Clone)]
pub struct AppContext {
    pub chat_hub: crate::api::chat::hub::ChatHub,
}

impl std::fmt::Debug for AppContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppContext").finish()
    }
}

#[derive(Clone, Debug)]
struct RuntimeConfig {
    binding: String,
    port: u16,
}

fn resolve_binding() -> String {
    std::env::var("SERVER_BINDING")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            if cfg!(test) {
                "127.0.0.1".to_string()
            } else {
                "0.0.0.0".to_string()
            }
        })
}

fn resolve_port() -> crate::error::AppResult<u16> {
    if let Ok(raw) = std::env::var("PORT") {
        return raw
            .parse::<u16>()
            .map_err(|e| crate::error::AppError::Message(format!("invalid PORT: {e}")));
    }
    Ok(5150)
}

fn init_tracing_once() -> crate::error::AppResult<()> {
    use std::sync::OnceLock;

    static TRACING_INIT: OnceLock<()> = OnceLock::new();
    if TRACING_INIT.get().is_some() {
        return Ok(());
    }

    let filter = match tracing_subscriber::EnvFilter::try_from_default_env() {
        Ok(f) => f,
        Err(e) if std::env::var("RUST_LOG").is_ok() => {
            return Err(crate::error::AppError::Message(format!(
                "invalid RUST_LOG: {e}"
            )));
        }
        Err(_) => tracing_subscriber::EnvFilter::new("info"),
    };

    let is_production = std::env::var("RUST_ENV").unwrap_or_default() == "production";

    if is_production {
        tracing_subscriber::fmt()
            .json()
            .flatten_event(true)
            .with_env_filter(filter)
            .with_target(true)
            .with_current_span(true)
            .with_span_list(true)
            .try_init()
            .map_err(|e| crate::error::AppError::Message(format!("logger init: {e}")))?;
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(true)
            .try_init()
            .map_err(|e| crate::error::AppError::Message(format!("logger init: {e}")))?;
    }

    let _ = TRACING_INIT.set(());
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigterm = match signal(SignalKind::terminate()) {
            Ok(stream) => stream,
            Err(error) => {
                tracing::warn!(%error, "failed to install SIGTERM handler; falling back to ctrl_c");
                let _ = tokio::signal::ctrl_c().await;
                return;
            }
        };

        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = sigterm.recv() => {}
        }
    }

    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

fn load_runtime_config() -> crate::error::AppResult<RuntimeConfig> {
    let binding = resolve_binding();
    let port = resolve_port()?;

    if std::env::var("DATABASE_URL").unwrap_or_default().is_empty() {
        return Err(crate::error::AppError::message("DATABASE_URL must be set"));
    }

    if std::env::var("JWT_SECRET").unwrap_or_default().is_empty() {
        return Err(crate::error::AppError::message("JWT_SECRET must be set"));
    }

    Ok(RuntimeConfig { binding, port })
}

#[derive(Clone, Copy)]
enum PoolRole {
    Api,
    Worker,
}

impl PoolRole {
    const fn env_var(self) -> &'static str {
        match self {
            Self::Api => "API_DATABASE_URL",
            Self::Worker => "WORKER_DATABASE_URL",
        }
    }

    const fn role_env_var(self) -> &'static str {
        match self {
            Self::Api => "DB_ROLE_API",
            Self::Worker => "DB_ROLE_WORKER",
        }
    }

    const fn default_db_user(self) -> &'static str {
        match self {
            Self::Api => "poziomki_api",
            Self::Worker => "poziomki_worker",
        }
    }

    fn expected_db_user(self) -> String {
        std::env::var(self.role_env_var())
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| self.default_db_user().to_string())
    }
}

fn is_production() -> bool {
    std::env::var("RUST_ENV").unwrap_or_default() == "production"
}

// The pool URL is chosen per-process so the API and worker can connect as
// least-privilege roles (poziomki_api / poziomki_worker) while migrations
// keep using the owner role via DATABASE_URL.
//
// In dev/test we fall back to DATABASE_URL when the role-specific var is
// unset. In production an empty role-specific var is a hard failure: the
// fallback would silently connect as the owner role (BYPASSRLS), which
// invalidates every RLS policy we ship.
fn resolve_pool_url(role: PoolRole) -> crate::error::AppResult<String> {
    let primary = role.env_var();
    if let Ok(value) = std::env::var(primary) {
        if !value.trim().is_empty() {
            return Ok(value);
        }
    }
    if is_production() {
        return Err(crate::error::AppError::message(format!(
            "{primary} must be set in production (no fallback to DATABASE_URL — the owner \
             role bypasses RLS)"
        )));
    }
    std::env::var("DATABASE_URL")
        .map_err(|_| crate::error::AppError::message("DATABASE_URL must be set"))
}

fn init_diesel_pool(role: PoolRole) -> crate::error::AppResult<()> {
    let url = resolve_pool_url(role)?;
    crate::db::init_pool(&url).map_err(crate::error::AppError::Message)
}

// In production, verify the pool actually connected as the expected
// least-privilege role. If pgdog / env / password misconfig routes us to
// the owner role instead, RLS is silently disabled — fail startup.
#[derive(diesel::deserialize::QueryableByName)]
struct CurrentDbUser {
    #[diesel(sql_type = diesel::sql_types::Text)]
    current_user: String,
}

async fn assert_pool_role(role: PoolRole) -> crate::error::AppResult<()> {
    use diesel_async::RunQueryDsl;

    if !is_production() {
        return Ok(());
    }
    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Message(format!("pool role check: {e}")))?;
    let row: CurrentDbUser = diesel::sql_query("SELECT current_user::text AS current_user")
        .get_result(&mut conn)
        .await
        .map_err(|e| crate::error::AppError::Message(format!("pool role check: {e}")))?;
    let expected = role.expected_db_user();
    if row.current_user != expected.as_str() {
        return Err(crate::error::AppError::Message(format!(
            "pool connected as db user {actual:?}, expected {expected:?} — refusing to start \
             (RLS would be bypassed)",
            actual = row.current_user,
        )));
    }
    tracing::info!(db_user = %row.current_user, "pool role verified");
    assert_pg_max_connections(&mut conn).await;
    Ok(())
}

// Pgdog is sized for 20 connections per role × 3 roles = 60. If postgres
// `max_connections` drifts below that (e.g. an old ALTER SYSTEM override in
// `postgresql.auto.conf` that survives a conf-file bump), pgdog can't realize
// its configured pool and the API silently runs with a tighter cap. Profiling
// in 2026-05 caught a 30-vs-60 drift this way. Warn loudly on boot so it's
// visible in logs without failing startup.
#[derive(diesel::deserialize::QueryableByName)]
struct MaxConnectionsRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    setting: String,
}

const PG_MIN_MAX_CONNECTIONS: i32 = 60;

async fn assert_pg_max_connections(conn: &mut crate::db::DbConn) {
    use diesel_async::RunQueryDsl;

    let row: Result<MaxConnectionsRow, _> =
        diesel::sql_query("SELECT setting FROM pg_settings WHERE name = 'max_connections'")
            .get_result(conn)
            .await;
    match row.map(|r| r.setting.parse::<i32>()) {
        Ok(Ok(value)) if value < PG_MIN_MAX_CONNECTIONS => {
            tracing::warn!(
                max_connections = value,
                expected_min = PG_MIN_MAX_CONNECTIONS,
                "postgres max_connections below pgdog pool ceiling — \
                 pool will be capped below configured size; check \
                 postgresql.auto.conf for an ALTER SYSTEM override"
            );
        }
        Ok(Ok(value)) => {
            tracing::info!(max_connections = value, "postgres max_connections verified");
        }
        Ok(Err(error)) => {
            tracing::warn!(%error, "could not parse max_connections setting");
        }
        Err(error) => {
            tracing::warn!(%error, "max_connections probe failed");
        }
    }
}

fn build_app_context(role: PoolRole) -> crate::error::AppResult<AppContext> {
    let migration_url = std::env::var("DATABASE_URL")
        .map_err(|_| crate::error::AppError::message("DATABASE_URL must be set"))?;
    crate::db::run_migrations(&migration_url).map_err(crate::error::AppError::Message)?;
    init_diesel_pool(role)?;
    Ok(AppContext {
        chat_hub: crate::api::chat::hub::ChatHub::new(),
    })
}

pub fn build_test_app_context() -> crate::error::AppResult<AppContext> {
    build_app_context(PoolRole::Api)
}

pub async fn reset_test_database() -> crate::error::AppResult<()> {
    crate::app_support::truncate_all_tables().await
}

pub fn build_router_with_state(ctx: AppContext) -> axum::Router {
    crate::api::router().with_state(ctx)
}

pub async fn run_api_server() -> crate::error::AppResult<()> {
    let _ = dotenvy::dotenv();
    init_tracing_once()?;
    crate::telemetry::init_metrics_exporter(crate::telemetry::ProcessKind::Api)?;
    crate::moderation::init_from_env()
        .map_err(|e| crate::error::AppError::Message(format!("moderation init: {e}")))?;
    crate::moderation::init_image_from_env()
        .map_err(|e| crate::error::AppError::Message(format!("image moderation init: {e}")))?;
    let cfg = load_runtime_config()?;
    let ctx = build_app_context(PoolRole::Api)?;
    assert_pool_role(PoolRole::Api).await?;
    let router = build_router_with_state(ctx);

    let listener = tokio::net::TcpListener::bind((cfg.binding.as_str(), cfg.port))
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;
    let addr = listener.local_addr().map_or_else(
        |_| format!("{}:{}", cfg.binding, cfg.port),
        |a| a.to_string(),
    );

    tracing::info!(addr = %addr, "Poziomki API server started");
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;
    tracing::info!("Poziomki API server stopping");
    Ok(())
}

pub async fn run_outbox_worker_process() -> crate::error::AppResult<()> {
    let _ = dotenvy::dotenv();
    init_tracing_once()?;
    crate::telemetry::init_metrics_exporter(crate::telemetry::ProcessKind::Worker)?;
    crate::moderation::init_from_env()
        .map_err(|e| crate::error::AppError::Message(format!("moderation init: {e}")))?;
    // Image moderation is upload-path only — the worker has no upload
    // handlers, so loading the ~22 MB ONNX session here would waste RSS.
    // If the outbox ever grows an image-scan job, re-add the init.
    let _cfg = load_runtime_config()?;
    let ctx = build_app_context(PoolRole::Worker)?;
    assert_pool_role(PoolRole::Worker).await?;

    crate::jobs::start_background_workers(&ctx)?;
    tracing::info!("Poziomki outbox worker process started");
    shutdown_signal().await;
    tracing::info!("Poziomki outbox worker process stopping");
    Ok(())
}
