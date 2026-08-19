use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

use ggs_core::action::Action;
use ggs_core::observation::PlayerView;
use ggs_core::strategy_trait::Strategy;
use ggs_core::variant::Variant;

/// Strategy that picks uniformly at random from legal actions.
/// Useful as a baseline and for MCTS rollouts.
#[allow(dead_code)]
pub struct RandomStrategy {
    rng: SmallRng,
}

impl RandomStrategy {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: SmallRng::seed_from_u64(seed),
        }
    }

    pub fn new_unseeded() -> Self {
        Self {
            rng: SmallRng::from_os_rng(),
        }
    }
}

impl Strategy for RandomStrategy {
    fn choose_action(&mut self, view: &PlayerView) -> Action {
        let n = view.legal_actions.len();
        if n == 0 {
            panic!("no legal actions");
        }

        let idx = self.rng.random_range(0..n);
        view.legal_actions[idx]
    }

    fn on_game_start(&mut self, _figure_id: u8, _variant: Variant) {}
    fn on_game_end(&mut self, _won: bool) {}
}
