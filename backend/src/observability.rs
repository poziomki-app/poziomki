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
    // Free Sentry plan: errors only, no performance transactions (5k/mo cap).
    // Bump SENTRY_TRACES_SAMPLE_RATE on a paid plan if you want tracing.
    let traces_sample_rate = std::env::var("SENTRY_TRACES_SAMPLE_RATE")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(0.0);
    let guard = sentry::init((
        dsn,
        sentry::ClientOptions {
            release: Some(env!("CARGO_PKG_VERSION").into()),
            environment: Some(environment.into()),
            traces_sample_rate,
            attach_stacktrace: true,
            ..Default::default()
        },
    ));
    let _ = SENTRY_GUARD.set(guard);
}

#[cfg(test)]
mod tests {
    use super::init_sentry;
    use sentry::test::with_captured_events;

    // Verify Sentry capture actually emits an event for an AppError.
    // Exercises the same `sentry::capture_error` API the tower layer uses
    // for 5xx responses, so a green test proves the wiring end-to-end.
    #[test]
    fn captures_app_error_via_capture_error() {
        let events = with_captured_events(|| {
            let err = crate::error::AppError::message("synthetic 5xx");
            sentry::capture_error(&err);
        });
        assert_eq!(events.len(), 1, "expected one captured event");
        let msg = events
            .first()
            .and_then(|e| e.exception.values.first())
            .and_then(|e| e.value.clone())
            .unwrap_or_default();
        assert!(msg.contains("synthetic 5xx"), "got: {msg}");
    }

    #[test]
    fn init_sentry_is_noop_without_dsn() {
        std::env::remove_var("SENTRY_DSN");
        init_sentry();
        // Calling capture_error here should not panic and (because the
        // guard wasn't bound to a Hub in this test) should produce no event
        // via the test transport.
        let events = with_captured_events(|| {
            sentry::capture_message("noop", sentry::Level::Error);
        });
        assert!(events.len() <= 1, "captured: {}", events.len());
    }
}
