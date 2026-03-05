use axum::http::{HeaderName, HeaderValue};
use axum_test::TestServer;
use serial_test::serial;

const TEST_OPS_TOKEN: &str = "test-metrics-secret-42";

fn setup() -> TestServer {
    std::env::set_var("OPS_STATUS_TOKEN", TEST_OPS_TOKEN);
    TestServer::new(poziomki_backend::app::build_metrics_test_router())
}

fn ops_header(token: &str) -> (HeaderName, HeaderValue) {
    (
        HeaderName::from_static("x-ops-token"),
        HeaderValue::from_str(token).unwrap(),
    )
}

#[tokio::test]
#[serial]
async fn metrics_rejects_missing_token() {
    let server = setup();
    let response = server.get("/api/v1/metrics").await;
    assert_eq!(response.status_code(), 401);
}

#[tokio::test]
#[serial]
async fn metrics_rejects_wrong_token() {
    let server = setup();
    let (key, val) = ops_header("wrong-token");
    let response = server.get("/api/v1/metrics").add_header(key, val).await;
    assert_eq!(response.status_code(), 401);
}

#[tokio::test]
#[serial]
async fn metrics_rejects_prefix_of_valid_token() {
    let server = setup();
    let partial = &TEST_OPS_TOKEN[..10];
    let (key, val) = ops_header(partial);
    let response = server.get("/api/v1/metrics").add_header(key, val).await;
    assert_eq!(response.status_code(), 401);
}

#[tokio::test]
#[serial]
async fn metrics_rejects_token_with_extra_suffix() {
    let server = setup();
    let extended = format!("{TEST_OPS_TOKEN}-extra");
    let (key, val) = ops_header(&extended);
    let response = server.get("/api/v1/metrics").add_header(key, val).await;
    assert_eq!(response.status_code(), 401);
}

#[tokio::test]
#[serial]
async fn metrics_rejects_when_token_not_configured() {
    std::env::remove_var("OPS_STATUS_TOKEN");
    let server = TestServer::new(poziomki_backend::app::build_metrics_test_router());
    let (key, val) = ops_header("any-token");
    let response: axum_test::TestResponse =
        server.get("/api/v1/metrics").add_header(key, val).await;
    assert_eq!(response.status_code(), 401);
}

#[tokio::test]
#[serial]
async fn metrics_returns_json_with_valid_token() {
    let server = setup();
    let (key, val) = ops_header(TEST_OPS_TOKEN);
    let response = server.get("/api/v1/metrics").add_header(key, val).await;
    assert_eq!(response.status_code(), 200);

    let payload: serde_json::Value = response.json();
    let sections = payload["charts"].as_array().expect("charts should be array");
    assert_eq!(sections.len(), 8);
}

#[tokio::test]
#[serial]
async fn metrics_supports_range_param() {
    let server = setup();
    let (key, val) = ops_header(TEST_OPS_TOKEN);
    let response = server
        .get("/api/v1/metrics?range=1h")
        .add_header(key, val)
        .await;
    assert_eq!(response.status_code(), 200);

    let payload: serde_json::Value = response.json();
    assert!(payload["charts"].is_array());
}

#[tokio::test]
#[serial]
async fn metrics_series_have_correct_shape() {
    let server = setup();
    let (key, val) = ops_header(TEST_OPS_TOKEN);
    let response = server.get("/api/v1/metrics").add_header(key, val).await;
    assert_eq!(response.status_code(), 200);

    let payload: serde_json::Value = response.json();
    let sections = payload["charts"].as_array().unwrap();

    for section in sections {
        assert!(section["label"].is_string(), "section missing label");
        let series = section["series"].as_array().expect("series should be array");
        assert!(!series.is_empty(), "section should have at least one series");

        for s in series {
            assert!(s["name"].is_string(), "series missing name");
            assert!(s["timestamps"].is_array(), "series missing timestamps");
            assert!(s["values"].is_array(), "series missing values");
        }
    }
}
