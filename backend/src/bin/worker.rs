use loco_rs::{
    app::Hooks,
    boot::{shutdown_signal, StartMode},
    environment::{resolve_from_env, Environment},
    logger,
};
use poziomki_backend::{app::App, tasks};

#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    let environment: Environment = resolve_from_env().into();
    let config = App::load_config(&environment).await?;
    let boot = App::boot(StartMode::ServerOnly, &environment, config).await?;

    if !App::init_logger(&boot.app_context)? {
        logger::init::<App>(&boot.app_context.config.logger)?;
    }

    tasks::start_background_workers(&boot.app_context)?;
    tracing::info!("Poziomki outbox worker process started");
    shutdown_signal().await;
    tracing::info!("Poziomki outbox worker process stopping");
    Ok(())
}
