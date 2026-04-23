//! Text moderation via ONNX Runtime inference against the Bielik-Guard model.

mod engine;
mod global;
mod scores;

pub use engine::{ModerationEngine, ModerationError};
pub use global::{init_from_env, shared};
pub use scores::{Category, Scores, Thresholds, Verdict};
