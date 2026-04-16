#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if std::env::args().nth(1).as_deref() == Some("healthcheck") {
        poziomki_backend::healthcheck::worker_healthcheck();
    }
    poziomki_backend::app::run_outbox_worker_process()
        .await
        .map_err(Into::into)
}
