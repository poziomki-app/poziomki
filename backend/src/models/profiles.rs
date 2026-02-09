use sea_orm::entity::prelude::*;

pub use super::_entities::profiles::{self, ActiveModel, Entity, Model};

impl ActiveModelBehavior for ActiveModel {}
