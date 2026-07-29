pub mod action;
pub mod board;
pub mod engine;
pub mod observation;
pub mod rules;
pub mod state;
pub mod strategy_trait;
pub mod variant;

pub mod prelude {
    pub use crate::action::Action;
    pub use crate::engine::{Game, GameLog, GameResult, StepResult};
    pub use crate::observation::PlayerView;
    pub use crate::state::GameState;
    pub use crate::strategy_trait::Strategy;
    pub use crate::variant::Variant;
}
