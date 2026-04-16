use std::process::exit;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::jobs::outbox::OUTBOX_WORKER_HEARTBEAT_PATH;

const HTTP_TIMEOUT: Duration = Duration::from_secs(3);
const WORKER_HEARTBEAT_STALE_AFTER_SECS: u64 = 30;
const DEFAULT_API_PORT: &str = "5150";

pub async fn api_healthcheck() -> ! {
    let port = std::env::var("PORT").unwrap_or_else(|_| DEFAULT_API_PORT.to_string());
    let url = format!("http://127.0.0.1:{port}/health");

    let Ok(client) = reqwest::Client::builder().timeout(HTTP_TIMEOUT).build() else {
        exit(1);
    };

    let ok = matches!(client.get(url).send().await, Ok(resp) if resp.status().is_success());
    exit(i32::from(!ok));
}

pub fn worker_healthcheck() -> ! {
    let Ok(contents) = std::fs::read_to_string(OUTBOX_WORKER_HEARTBEAT_PATH) else {
        exit(1);
    };
    let Ok(ts_secs) = contents.trim().parse::<u64>() else {
        exit(1);
    };
    let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        exit(1);
    };
    let age_secs = now.as_secs().saturating_sub(ts_secs);
    exit(i32::from(age_secs >= WORKER_HEARTBEAT_STALE_AFTER_SECS))
}
