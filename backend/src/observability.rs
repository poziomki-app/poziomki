use std::sync::OnceLock;

static SENTRY_GUARD: OnceLock<sentry::ClientInitGuard> = OnceLock::new();

pub fn init_sentry() {
    let Ok(dsn) = std::env::var("SENTRY_DSN") else {
        return;
    };
    if dsn.trim().is_empty() {
        return;
    }
    let environment = std::env::var("RUST_ENV").unwrap_or_else(|_| "development".to_string());
    let guard = sentry::init((
        dsn,
        sentry::ClientOptions {
            release: Some(env!("CARGO_PKG_VERSION").into()),
            environment: Some(environment.into()),
            traces_sample_rate: 0.1,
            attach_stacktrace: true,
            ..Default::default()
        },
    ));
    let _ = SENTRY_GUARD.set(guard);
}
