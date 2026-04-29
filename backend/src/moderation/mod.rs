//! Moderation engines (text + image).
//!
//! Text moderation runs the Bielik-Guard model against free-text fields
//! (bios, event descriptions, chat). Image moderation runs the Marqo
//! `nsfw-image-detection-384` model against user uploads. Both engines
//! are independent process-wide singletons.

mod engine;
mod global;
mod image_engine;
mod image_global;
mod scores;

pub use engine::{ModerationEngine, ModerationError};
pub use global::{init_from_env, shared};
pub use image_engine::{ImageModerationEngine, ImageModerationError, ImageScore, ImageVerdict};
pub use image_global::{init_image_from_env, shared_image};
pub use scores::{Category, Scores, Thresholds, Verdict};
