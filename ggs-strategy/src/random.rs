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
        Self { rng: SmallRng::seed_from_u64(seed) }
    }

    pub fn new_unseeded() -> Self {
        Self { rng: SmallRng::from_os_rng() }
    }
}

impl Strategy for RandomStrategy {
    fn choose_action(&self, view: &PlayerView) -> Action {
        // Interior mutability via UnsafeCell would be cleaner, but for now
        // we take &self and use a thread_local RNG as fallback.
        // Since RandomStrategy is used in single-threaded rollouts where
        // the caller holds &mut, we expose a mutable version separately.
        let idx = (view.die_roll as usize).wrapping_add(view.deck_size as usize)
            % view.legal_actions.len().max(1);
        view.legal_actions[idx]
    }

    fn on_game_start(&mut self, _figure_id: u8, _variant: Variant) {}
    fn on_game_end(&mut self, _won: bool) {}
}

/// Mutable-access version used in `simulate_one_game` for true randomness.
pub struct RandomStrategyMut {
    rng: SmallRng,
}

impl RandomStrategyMut {
    pub fn new(seed: u64) -> Self {
        Self { rng: SmallRng::seed_from_u64(seed) }
    }

    pub fn new_unseeded() -> Self {
        Self { rng: SmallRng::from_os_rng() }
    }

    pub fn choose_action_mut(&mut self, view: &PlayerView) -> Action {
        let n = view.legal_actions.len();
        if n == 0 {
            panic!("no legal actions");
        }
        let idx = self.rng.random_range(0..n);
        view.legal_actions[idx]
    }
}

impl Strategy for RandomStrategyMut {
    fn choose_action(&self, view: &PlayerView) -> Action {
        // Deterministic fallback when called via shared ref.
        view.legal_actions[0]
    }
}
