use sea_orm::entity::prelude::*;

pub use super::_entities::user_settings::{self, ActiveModel, Entity, Model};

impl ActiveModelBehavior for ActiveModel {}
