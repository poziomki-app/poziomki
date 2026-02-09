use sea_orm::entity::prelude::*;

pub use super::_entities::profile_tags::{self, ActiveModel, Entity, Model};

impl ActiveModelBehavior for ActiveModel {}
