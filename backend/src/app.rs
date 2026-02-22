#[derive(Clone, Debug)]
pub struct AppContext {}

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

    let level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let filter = tracing_subscriber::EnvFilter::try_new(level)
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init()
        .map_err(|e| crate::error::AppError::Message(format!("logger init failed: {e}")))?;

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

fn init_diesel_pool() -> crate::error::AppResult<()> {
    let url = std::env::var("DATABASE_URL")
        .map_err(|_| crate::error::AppError::message("DATABASE_URL must be set"))?;
    crate::db::init_pool(&url).map_err(crate::error::AppError::Message)
}

fn build_app_context() -> crate::error::AppResult<AppContext> {
    init_diesel_pool()?;
    Ok(AppContext {})
}

pub fn build_test_app_context() -> crate::error::AppResult<AppContext> {
    build_app_context()
}

pub async fn reset_test_database() -> crate::error::AppResult<()> {
    crate::app_support::truncate_all_tables().await
}

pub fn build_router_with_state(ctx: AppContext) -> axum::Router {
    crate::controllers::migration_api::router().with_state(ctx)
}

pub async fn run_api_server() -> crate::error::AppResult<()> {
    let _ = dotenvy::dotenv();
    init_tracing_once()?;
    let cfg = load_runtime_config()?;
    let ctx = build_app_context()?;
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
    let _cfg = load_runtime_config()?;
    let ctx = build_app_context()?;

    crate::tasks::start_background_workers(&ctx)?;
    tracing::info!("Poziomki outbox worker process started");
    shutdown_signal().await;
    tracing::info!("Poziomki outbox worker process stopping");
    Ok(())
}
