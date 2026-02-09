use loco_rs::Result;
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};

const TRUNCATE_ORDER: &[&str] = &[
    "event_attendees",
    "event_tags",
    "profile_tags",
    "events",
    "uploads",
    "user_settings",
    "sessions",
    "profiles",
    "degrees",
    "tags",
    "users",
];

pub async fn truncate_all_tables(db: &DatabaseConnection) -> Result<()> {
    for table in TRUNCATE_ORDER {
        db.execute(Statement::from_string(
            db.get_database_backend(),
            format!("DELETE FROM \"{table}\""),
        ))
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;
    }
    Ok(())
}
