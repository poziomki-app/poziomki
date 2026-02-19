use async_trait::async_trait;
use loco_rs::{
    app::{AppContext, Hooks, Initializer},
    bgworker::Queue,
    boot::{create_app, BootResult, StartMode},
    config::Config,
    controller::AppRoutes,
    db,
    environment::Environment,
    task::Tasks,
    Result,
};
use migration::Migrator;
use std::path::Path;

#[allow(unused_imports)]
use crate::{app_support, controllers, models::_entities::users, tasks};

pub struct App;
#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }

    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: Config,
    ) -> Result<BootResult> {
        if environment == &Environment::Production {
            for var in ["DATABASE_URL", "JWT_SECRET"] {
                if std::env::var(var).unwrap_or_default().is_empty() {
                    return Err(loco_rs::Error::Message(format!(
                        "{var} must be set in production"
                    )));
                }
            }
            if std::env::var("ALLOWED_EMAIL_DOMAIN")
                .unwrap_or_default()
                .is_empty()
            {
                tracing::warn!(
                    "ALLOWED_EMAIL_DOMAIN not set — defaulting to example.com (registration will be restricted)"
                );
            }
        }
        let boot = create_app::<Self, Migrator>(mode, environment, config).await?;
        Ok(boot)
    }

    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![])
    }

    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::empty().add_routes(controllers::migration_api::routes())
    }
    async fn connect_workers(_ctx: &AppContext, _queue: &Queue) -> Result<()> {
        Ok(())
    }

    fn register_tasks(_tasks: &mut Tasks) {}
    async fn truncate(ctx: &AppContext) -> Result<()> {
        app_support::truncate_all_tables(&ctx.db).await?;
        Ok(())
    }
    async fn seed(ctx: &AppContext, base: &Path) -> Result<()> {
        db::seed::<users::ActiveModel>(&ctx.db, &base.join("users.yaml").display().to_string())
            .await?;
        Ok(())
    }
}
