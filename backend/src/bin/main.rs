#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if std::env::args().nth(1).as_deref() == Some("healthcheck") {
        poziomki_backend::healthcheck::api_healthcheck().await;
    }
    poziomki_backend::app::run_api_server()
        .await
        .map_err(Into::into)
}
