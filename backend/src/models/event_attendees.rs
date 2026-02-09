use sea_orm::entity::prelude::*;

pub use super::_entities::event_attendees::{self, ActiveModel, Entity, Model};

impl ActiveModelBehavior for ActiveModel {}
