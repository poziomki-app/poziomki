use axum::{body::Body, extract::Request, middleware::Next, response::Response};
use std::sync::atomic::Ordering;
use std::time::Instant;

use super::collector::EndpointGroup;

/// Axum middleware function for request tracking.
///
/// Use with `axum::middleware::from_fn(metrics_middleware)`.
pub async fn metrics_middleware(req: Request<Body>, next: Next) -> Response {
    let path = req.uri().path().to_owned();

    // Skip self-measurement for metrics endpoints
    if path.starts_with("/api/v1/metrics") {
        return next.run(req).await;
    }

    let group = EndpointGroup::from_path(&path);
    let start = Instant::now();

    let response = next.run(req).await;

    if let Some(m) = super::metrics() {
        let elapsed = start.elapsed();
        let status = response.status().as_u16();

        m.req_total.fetch_add(1, Ordering::Relaxed);
        if (400..500).contains(&status) {
            m.req_4xx.fetch_add(1, Ordering::Relaxed);
        } else if status >= 500 {
            m.req_5xx.fetch_add(1, Ordering::Relaxed);
        }

        if let Some(h) = m.latency_for_group(group) {
            h.record(elapsed);
        }
    }

    response
}
