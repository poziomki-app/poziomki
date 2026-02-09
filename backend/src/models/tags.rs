use sea_orm::entity::prelude::*;

pub use super::_entities::tags::{self, ActiveModel, Entity, Model};

impl ActiveModelBehavior for ActiveModel {}
