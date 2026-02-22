use sea_orm::{
    ActiveModelBehavior, ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait,
    DatabaseConnection, DbErr, EntityTrait, QueryFilter, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

pub use super::_entities::users::{self, ActiveModel, Entity, Model};
use crate::security;

#[derive(Debug)]
pub enum ModelError {
    EntityNotFound,
    EntityAlreadyExists,
    Validation(String),
    Any(Box<dyn std::error::Error + Send + Sync>),
}

impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EntityNotFound => write!(f, "entity not found"),
            Self::EntityAlreadyExists => write!(f, "entity already exists"),
            Self::Validation(msg) => write!(f, "validation error: {msg}"),
            Self::Any(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for ModelError {}

impl From<DbErr> for ModelError {
    fn from(value: DbErr) -> Self {
        Self::Any(value.into())
    }
}

impl From<jsonwebtoken::errors::Error> for ModelError {
    fn from(value: jsonwebtoken::errors::Error) -> Self {
        Self::Any(value.into())
    }
}

pub type ModelResult<T> = std::result::Result<T, ModelError>;

#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterParams {
    pub email: String,
    pub password: String,
    pub name: String,
}

#[derive(Debug, Validate, Deserialize)]
struct ValidatorInput {
    #[validate(length(min = 1, message = "Name is required."))]
    pub name: String,
    #[validate(email(message = "invalid email"))]
    pub email: String,
}

fn validate_active_model(model: &ActiveModel) -> std::result::Result<(), DbErr> {
    let input = ValidatorInput {
        name: model.name.as_ref().to_owned(),
        email: model.email.as_ref().to_owned(),
    };
    input
        .validate()
        .map_err(|e| DbErr::Custom(format!("validation failed: {e}")))
}

#[async_trait::async_trait]
impl ActiveModelBehavior for super::_entities::users::ActiveModel {
    async fn before_save<C>(self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        validate_active_model(&self)?;
        if insert {
            let mut this = self;
            this.pid = ActiveValue::Set(Uuid::new_v4());
            this.api_key = ActiveValue::Set(format!("lo-{}", Uuid::new_v4()));
            Ok(this)
        } else {
            Ok(self)
        }
    }
}

impl Model {
    pub async fn find_by_email(db: &DatabaseConnection, email: &str) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(users::Column::Email.eq(email))
            .one(db)
            .await?;
        user.ok_or(ModelError::EntityNotFound)
    }

    pub async fn find_by_pid(db: &DatabaseConnection, pid: &str) -> ModelResult<Self> {
        let parse_uuid = Uuid::parse_str(pid).map_err(|e| ModelError::Any(e.into()))?;
        let user = users::Entity::find()
            .filter(users::Column::Pid.eq(parse_uuid))
            .one(db)
            .await?;
        user.ok_or(ModelError::EntityNotFound)
    }

    #[must_use]
    pub fn verify_password(&self, password: &str) -> bool {
        security::verify_password(password, &self.password)
    }

    pub async fn create_with_password(
        db: &DatabaseConnection,
        params: &RegisterParams,
    ) -> ModelResult<Self> {
        let txn = db.begin().await?;

        if users::Entity::find()
            .filter(users::Column::Email.eq(&params.email))
            .one(&txn)
            .await?
            .is_some()
        {
            return Err(ModelError::EntityAlreadyExists);
        }

        let password_hash = security::hash_password(&params.password)
            .map_err(|e| ModelError::Validation(e.to_string()))?;
        let user = users::ActiveModel {
            email: ActiveValue::set(params.email.clone()),
            password: ActiveValue::set(password_hash),
            name: ActiveValue::set(params.name.clone()),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        txn.commit().await?;

        Ok(user)
    }

    pub fn generate_jwt(&self, secret: &str, expiration: u64) -> ModelResult<String> {
        security::generate_user_jwt(secret, expiration, self.pid.to_string())
            .map_err(ModelError::from)
    }
}
