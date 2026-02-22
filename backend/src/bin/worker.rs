#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    poziomki_backend::app::run_outbox_worker_process()
        .await
        .map_err(Into::into)
}
