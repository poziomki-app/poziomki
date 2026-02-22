#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    poziomki_backend::app::run_api_server()
        .await
        .map_err(Into::into)
}
