use axum_test::TestServer;
use serial_test::serial;

fn build_server() -> TestServer {
    build_server_with_token(Some("test-metrics-token"))
}

fn build_server_with_token(token: Option<&str>) -> TestServer {
    match token {
        Some(token) => std::env::set_var("OPS_STATUS_TOKEN", token),
        None => std::env::remove_var("OPS_STATUS_TOKEN"),
    }
    let router = poziomki_backend::app::build_metrics_test_router();
    TestServer::new(router)
}

async fn get_metrics(server: &TestServer, path: &str) -> axum_test::TestResponse {
    server
        .get(path)
        .add_header("x-ops-token", "test-metrics-token")
        .await
}

#[tokio::test]
#[serial]
async fn rejects_missing_token() {
    let server = build_server();
    let res = server.get("/api/v1/metrics").await;
    res.assert_status_unauthorized();
}

#[tokio::test]
#[serial]
async fn rejects_wrong_token() {
    let server = build_server();
    let res = server
        .get("/api/v1/metrics")
        .add_header("x-ops-token", "wrong-token")
        .await;
    res.assert_status_unauthorized();
}

#[tokio::test]
#[serial]
async fn rejects_partial_token() {
    let server = build_server();
    let res = server
        .get("/api/v1/metrics")
        .add_header("x-ops-token", "test-metrics")
        .await;
    res.assert_status_unauthorized();
}

#[tokio::test]
#[serial]
async fn rejects_extended_token() {
    let server = build_server();
    let res = server
        .get("/api/v1/metrics")
        .add_header("x-ops-token", "test-metrics-token-extra")
        .await;
    res.assert_status_unauthorized();
}

#[tokio::test]
#[serial]
async fn accepts_valid_token_and_returns_charts() {
    let server = build_server();
    let res = get_metrics(&server, "/api/v1/metrics").await;
    res.assert_status_ok();

    let body: serde_json::Value = res.json();

    // Verify meta fields
    let meta = &body["meta"];
    assert_eq!(meta["source"], "memory");
    assert_eq!(meta["degraded"], true);
    assert!(meta["sample_interval_seconds"].is_number());
    assert!(meta["generated_at_epoch"].is_number());
    assert!(meta["last_sample_epoch"].is_number());
    assert!(meta["sample_failures_total"].is_number());

    // Verify 8 charts
    let charts = body["charts"].as_array().expect("charts array");
    assert_eq!(charts.len(), 8);
}

#[tokio::test]
#[serial]
async fn metrics_ranges_return_ok_with_memory_fallback() {
    let server = build_server();

    for path in [
        "/api/v1/metrics?range=1h",
        "/api/v1/metrics?range=6h",
        "/api/v1/metrics?range=24h",
        "/api/v1/metrics?range=unexpected",
    ] {
        let res = get_metrics(&server, path).await;
        res.assert_status_ok();

        let body: serde_json::Value = res.json();
        assert_eq!(body["meta"]["source"], "memory", "{path}");
        assert_eq!(body["meta"]["degraded"], true, "{path}");
        assert_eq!(body["charts"].as_array().map(Vec::len), Some(8), "{path}");
    }
}

#[tokio::test]
#[serial]
async fn dashboard_returns_html_with_valid_token() {
    let server = build_server();
    let res = server
        .get("/api/v1/metrics/?token=test-metrics-token")
        .await;
    res.assert_status_ok();

    let body = res.text();
    assert!(body.contains("<!DOCTYPE html>") || body.contains("<html"));
}

#[tokio::test]
#[serial]
async fn dashboard_rejects_invalid_token() {
    let server = build_server();
    let res = server.get("/api/v1/metrics/?token=wrong").await;
    res.assert_status_unauthorized();
}

#[tokio::test]
#[serial]
async fn dashboard_returns_not_found_without_ops_token() {
    let server = build_server_with_token(None);
    let res = server
        .get("/api/v1/metrics/?token=test-metrics-token")
        .await;
    res.assert_status_not_found();
}
