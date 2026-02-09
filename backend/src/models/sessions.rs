use sea_orm::entity::prelude::*;

pub use super::_entities::sessions::{self, ActiveModel, Entity, Model};

impl ActiveModelBehavior for ActiveModel {}
