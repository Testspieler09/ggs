use ggs_core::action::Action;
use ggs_core::observation::PlayerView;
use ggs_core::strategy_trait::Strategy;
use ggs_core::variant::Variant;

/// Monte Carlo Tree Search strategy stub.
///
/// Intended algorithm: cooperative UCT.
/// Each node stores visit count and accumulated win value (0.0 / 1.0).
/// Rollouts use the random strategy.
/// Arena allocation (Vec<MctsNode> with integer child indices) avoids pointer chasing.
///
/// TODO: implement UCT selection, expansion, simulation, backpropagation.
pub struct MctsStrategy {
    /// Number of simulations per `choose_action` call.
    pub simulations: u32,
    /// UCT exploration constant (√2 is a common default).
    pub exploration: f32,
}

impl MctsStrategy {
    pub fn new(simulations: u32, exploration: f32) -> Self {
        Self {
            simulations,
            exploration,
        }
    }
}

/// A single node in the search tree (arena-indexed).
#[allow(dead_code)]
struct MctsNode {
    parent: u32,
    action: Option<Action>,
    visits: u32,
    value: f32,
    first_child: u32,
    child_count: u32,
}

impl Strategy for MctsStrategy {
    fn choose_action(&mut self, view: &PlayerView) -> Action {
        // Stub: fall back to first legal action until UCT is implemented.
        view.legal_actions
            .first()
            .copied()
            .expect("no legal actions")
    }

    fn on_game_start(&mut self, _figure_id: u8, _variant: Variant) {
        // Reset tree for new game.
    }
}
