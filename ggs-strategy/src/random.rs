use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::cell::UnsafeCell;

use ggs_core::action::Action;
use ggs_core::observation::PlayerView;
use ggs_core::strategy_trait::Strategy;
use ggs_core::variant::Variant;

// TODO: check why there is a mut and a immutable version of this and if we can reduce to one

/// Strategy that picks uniformly at random from legal actions.
/// Useful as a baseline and for MCTS rollouts.
#[allow(dead_code)]
pub struct RandomStrategy {
    rng: UnsafeCell<SmallRng>,
}

impl RandomStrategy {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: UnsafeCell::new(SmallRng::seed_from_u64(seed)),
        }
    }

    pub fn new_unseeded() -> Self {
        Self {
            rng: UnsafeCell::new(SmallRng::from_os_rng()),
        }
    }
}

impl Strategy for RandomStrategy {
    fn choose_action(&self, view: &PlayerView) -> Action {
        let n = view.legal_actions.len();
        if n == 0 {
            panic!("no legal actions");
        }
        // let idx = self.rng.random_range(0..n);
        // view.legal_actions[idx]
        view.legal_actions[0]
    }

    fn on_game_start(&mut self, _figure_id: u8, _variant: Variant) {}
    fn on_game_end(&mut self, _won: bool) {}
}
