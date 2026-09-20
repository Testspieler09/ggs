use ggs_core::action::Action;
use ggs_core::engine::{Game, StepResult};
use ggs_core::observation::PlayerView;
use ggs_core::state::GameState;
use ggs_core::strategy_trait::Strategy;
use ggs_core::variant::Variant;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

/// Monte Carlo Tree Search strategy using cooperative UCT.
///
/// Each node stores visit count and accumulated win value (0.0 / 1.0).
/// Rollouts use random play to terminal state via `Game::step`.
/// Arena allocation (Vec<MctsNode> with integer child indices) avoids pointer chasing.
pub struct MctsStrategy {
    /// Number of simulations per `choose_action` call.
    pub simulations: u32,
    /// UCT exploration constant (√2 is a common default).
    pub exploration: f32,
    rng: SmallRng,
}

impl MctsStrategy {
    pub fn new(simulations: u32, exploration: f32, seed: u64) -> Self {
        Self {
            simulations,
            exploration,
            rng: SmallRng::seed_from_u64(seed),
        }
    }

    pub fn new_unseeded(simulations: u32, exploration: f32) -> Self {
        Self {
            simulations,
            exploration,
            rng: SmallRng::from_os_rng(),
        }
    }

    /// Returns the arena index of the best child of `parent_idx` by UCT score.
    /// Unvisited children always score +∞ so they are selected first.
    fn select_child_by_uct(&self, arena: &[MctsNode], parent_idx: u32) -> u32 {
        let node = &arena[parent_idx as usize];
        let parent_visits = node.visits as f32;
        let first_child = node.first_child as usize;

        (0..node.child_count as usize)
            .map(|i| first_child + i)
            .max_by(|&a, &b| {
                let uct = |idx: usize| -> f32 {
                    let n = &arena[idx];
                    if n.visits == 0 {
                        return f32::INFINITY;
                    }
                    n.value / n.visits as f32
                        + self.exploration * (parent_visits.ln() / n.visits as f32).sqrt()
                };
                uct(a).partial_cmp(&uct(b)).unwrap()
            })
            .unwrap() as u32
    }

    fn is_fully_expanded(&self, arena: &[MctsNode], idx: u32, legal_count: usize) -> bool {
        arena[idx as usize].child_count as usize == legal_count
    }

    /// Appends one new child for the next unexplored action. Returns the child's index.
    fn expand(arena: &mut Vec<MctsNode>, parent_idx: u32, action: Action, state: GameState) -> u32 {
        let new_idx = arena.len() as u32;

        let parent = &mut arena[parent_idx as usize];
        if parent.child_count == 0 {
            parent.first_child = new_idx;
        }
        parent.child_count += 1;

        arena.push(MctsNode {
            parent: parent_idx,
            action: Some(action),
            state,
            visits: 0,
            value: 0.0,
            first_child: 0,
            child_count: 0,
        });

        new_idx
    }

    fn run_simulation(&self, arena: &mut Vec<MctsNode>, rng: &mut SmallRng) {
        // --- 1. Selection: walk down fully-expanded nodes by UCT ---
        let mut node_idx = 0u32;

        loop {
            let game = Game::from_state(arena[node_idx as usize].state.clone(), rng.random());
            let legal = game.legal_actions();
            if game.is_finished() || !self.is_fully_expanded(arena, node_idx, legal.len()) {
                break;
            }
            node_idx = self.select_child_by_uct(arena, node_idx);
        }

        // --- 2. Expansion: if not terminal, create one new child ---
        let mut game = Game::from_state(arena[node_idx as usize].state.clone(), rng.random());
        if !game.is_finished() {
            let legal = game.legal_actions();
            let expanded_count = arena[node_idx as usize].child_count as usize;
            if expanded_count < legal.len() {
                let action = legal[expanded_count];
                let mut child_game = Game::from_state(game.state_snapshot(), rng.random());
                child_game.step(action);
                let child_state = child_game.state_snapshot();
                node_idx = Self::expand(arena, node_idx, action, child_state);
                game = child_game;
            }
        }

        // --- 3. Rollout: random play to terminal ---
        let result = Self::random_rollout(game, rng);

        // --- 4. Backpropagation ---
        Self::backprop(arena, node_idx, result);
    }

    fn random_rollout(mut game: Game, rng: &mut SmallRng) -> f32 {
        loop {
            if game.is_finished() {
                return if game.winner() == Some(true) {
                    1.0
                } else {
                    0.0
                };
            }
            let legal = game.legal_actions();
            let action = legal[rng.random_range(0..legal.len())];
            match game.step(action) {
                StepResult::Win => return 1.0,
                StepResult::Loss => return 0.0,
                _ => {}
            }
        }
    }

    fn backprop(arena: &mut [MctsNode], mut idx: u32, result: f32) {
        loop {
            arena[idx as usize].visits += 1;
            arena[idx as usize].value += result;
            let parent = arena[idx as usize].parent;
            if parent == idx {
                break; // root's parent points to itself
            }
            idx = parent;
        }
    }

    /// Returns the root's child with the highest visit count (exploitation only).
    fn best_child_action(arena: &[MctsNode]) -> Action {
        let root = &arena[0];
        let first = root.first_child as usize;
        let best = (0..root.child_count as usize)
            .map(|i| first + i)
            .max_by_key(|&i| arena[i].visits)
            .unwrap();
        arena[best].action.unwrap()
    }

    fn view_to_state(view: &PlayerView) -> GameState {
        use ggs_core::state::EdgeState;

        let mut edge_states = [EdgeState::default(); ggs_core::board::EDGE_COUNT];
        for (i, &closed) in view.edge_closed.iter().enumerate() {
            edge_states[i].closed = closed;
        }

        GameState {
            variant: view.variant,
            figure_count: view.figure_count,
            active_figure: view.active_figure,
            phase: view.phase,
            die_roll: view.die_roll,
            moves_remaining: if matches!(view.phase, ggs_core::state::TurnPhase::Move) {
                view.die_roll
            } else {
                0
            },
            figure_pos: view.figure_positions,
            figure_carries: view.figure_carries,
            node_states: view.node_states,
            edge_states,
            spuk_count: view.spuk_count,
            jewel_deposited: view.jewel_deposited,
            jewel_number: view.jewel_number,
            next_required_jewel: view.next_required_jewel,
            deck: ggs_core::state::GhostDeck {
                cards: [ggs_core::state::GhostCard::Room(ggs_core::board::RoomLabel::A);
                    ggs_core::state::GHOST_DECK_CAPACITY],
                top: 0,
                size: view.deck_size,
            },
            game_over: false,
            players_won: false,
        }
    }
}

/// A single node in the MCTS search tree (arena-indexed).
struct MctsNode {
    parent: u32,
    action: Option<Action>,
    /// Game state at this node (after its action has been applied).
    state: GameState,
    visits: u32,
    value: f32,
    /// Index in the arena of the first child.
    first_child: u32,
    child_count: u32,
}

impl Strategy for MctsStrategy {
    fn choose_action(&mut self, view: &PlayerView) -> Action {
        let mut arena = vec![MctsNode {
            parent: 0, // root's parent points to itself
            action: None,
            state: Self::view_to_state(view),
            visits: 0,
            value: 0.0,
            first_child: 0,
            child_count: 0,
        }];

        for _ in 0..self.simulations {
            self.run_simulation(&mut arena, &mut self.rng.clone());
        }

        // Advance the stored rng so repeated calls diverge.
        let _: u64 = self.rng.random();

        Self::best_child_action(&arena)
    }

    fn on_game_start(&mut self, _figure_id: u8, _variant: Variant) {}
}
