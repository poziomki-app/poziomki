use sea_orm::entity::prelude::*;

pub use super::_entities::matrix_dm_rooms::{self, ActiveModel, Entity, Model};

impl ActiveModelBehavior for ActiveModel {}
