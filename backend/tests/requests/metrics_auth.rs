use axum_test::TestServer;

fn build_server() -> TestServer {
    std::env::set_var("OPS_STATUS_TOKEN", "test-metrics-token");
    let router = poziomki_backend::app::build_metrics_test_router();
    TestServer::new(router)
}

#[tokio::test]
async fn rejects_missing_token() {
    let server = build_server();
    let res = server.get("/api/v1/metrics").await;
    res.assert_status_unauthorized();
}

#[tokio::test]
async fn rejects_wrong_token() {
    let server = build_server();
    let res = server
        .get("/api/v1/metrics")
        .add_header("x-ops-token", "wrong-token")
        .await;
    res.assert_status_unauthorized();
}

#[tokio::test]
async fn rejects_partial_token() {
    let server = build_server();
    let res = server
        .get("/api/v1/metrics")
        .add_header("x-ops-token", "test-metrics")
        .await;
    res.assert_status_unauthorized();
}

#[tokio::test]
async fn rejects_extended_token() {
    let server = build_server();
    let res = server
        .get("/api/v1/metrics")
        .add_header("x-ops-token", "test-metrics-token-extra")
        .await;
    res.assert_status_unauthorized();
}

#[tokio::test]
async fn accepts_valid_token_and_returns_charts() {
    let server = build_server();
    let res = server
        .get("/api/v1/metrics")
        .add_header("x-ops-token", "test-metrics-token")
        .await;
    res.assert_status_ok();

    let body: serde_json::Value = res.json();

    // Verify meta fields
    let meta = &body["meta"];
    assert!(meta["source"].is_string());
    assert!(meta["sample_interval_seconds"].is_number());
    assert!(meta["generated_at_epoch"].is_number());

    // Verify 8 charts
    let charts = body["charts"].as_array().expect("charts array");
    assert_eq!(charts.len(), 8);
}

#[tokio::test]
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
async fn dashboard_rejects_invalid_token() {
    let server = build_server();
    let res = server.get("/api/v1/metrics/?token=wrong").await;
    res.assert_status_unauthorized();
}
