use crate::action::Action;
use crate::board::{NodeId, EDGE_COUNT, NODE_COUNT};
use crate::state::{FigureId, GameState, JewelId, NodeState, TurnPhase, JEWEL_COUNT, MAX_FIGURES};
use crate::variant::Variant;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

/// A player-visible snapshot of the game state.
/// Contains no hidden information: deck order is hidden (only count exposed).
/// Jewel numbers in the numbered-jewel variant are revealed only after a figure
/// has entered the room (non-zero in `jewel_number`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerView {
    pub figure_id: FigureId,
    pub figure_count: u8,
    pub active_figure: FigureId,
    pub phase: TurnPhase,
    pub die_roll: u8,
    pub figure_positions: [NodeId; MAX_FIGURES],
    pub figure_carries: [Option<JewelId>; MAX_FIGURES],
    /// Per-node state (same layout as GameState).
    #[serde(with = "BigArray")]
    pub node_states: [NodeState; NODE_COUNT],
    /// Per-edge closed flags.
    #[serde(with = "BigArray")]
    pub edge_closed: [bool; EDGE_COUNT],
    pub spuk_count: u8,
    pub jewel_deposited: [bool; JEWEL_COUNT],
    /// Jewel numbers: 0 = not yet revealed (or variant disabled).
    pub jewel_number: [u8; JEWEL_COUNT],
    pub next_required_jewel: u8,
    /// Number of cards remaining in the ghost deck (order is hidden).
    pub deck_size: u8,
    /// Legal actions available right now.
    pub legal_actions: Vec<Action>,
    pub variant: Variant,
}

impl PlayerView {
    pub fn from_state(state: &GameState, figure_id: FigureId, legal: &[Action]) -> Self {
        let mut edge_closed = [false; EDGE_COUNT];
        for (i, es) in state.edge_states.iter().enumerate() {
            edge_closed[i] = es.closed;
        }

        // In numbered-jewel variant, only expose jewel numbers that have been revealed.
        // A number is revealed (> 0) once a figure has entered the room.
        // We expose whatever is in state.jewel_number directly — the engine is responsible
        // for keeping unrevealed entries as 0 in the non-visible sense.
        // (The actual revelation happens in engine.rs when a figure enters a room.)
        let jewel_number = if state.variant.numbered_jewels {
            state.jewel_number
        } else {
            [0u8; JEWEL_COUNT]
        };

        Self {
            figure_id,
            figure_count: state.figure_count,
            active_figure: state.active_figure,
            phase: state.phase,
            die_roll: state.die_roll,
            figure_positions: state.figure_pos,
            figure_carries: state.figure_carries,
            node_states: state.node_states,
            edge_closed,
            spuk_count: state.spuk_count,
            jewel_deposited: state.jewel_deposited,
            jewel_number,
            next_required_jewel: state.next_required_jewel,
            deck_size: state.deck.remaining(),
            legal_actions: legal.to_vec(),
            variant: state.variant,
        }
    }
}

// ---------------------------------------------------------------------------
// ML observation vector
// ---------------------------------------------------------------------------

/// Returns the fixed dimension of the observation vector for any variant.
/// Call this to size neural network input layers.
pub fn observation_dim() -> usize {
    // figure positions: 4 figures × NODE_COUNT one-hot = 4 × 23 = 92
    // figure carry: 4 × (JEWEL_COUNT + 1) = 4 × 9 = 36
    // node ghosts: NODE_COUNT normalized = 23
    // node spuk: NODE_COUNT binary = 23
    // edge closed: EDGE_COUNT binary = 28
    // spuk count: 1 normalized
    // jewel in node: NODE_COUNT binary = 23
    // jewel deposited: JEWEL_COUNT binary = 8
    // jewel number revealed: JEWEL_COUNT normalized = 8
    // next required jewel: 1 normalized
    // deck size: 1 normalized
    // active figure one-hot: MAX_FIGURES = 4
    // phase one-hot: 6
    // die roll: 1 normalized
    4 * NODE_COUNT      // figure positions (one-hot)
        + 4 * (JEWEL_COUNT + 1) // carry status (0..=8)
        + NODE_COUNT    // ghosts per node
        + NODE_COUNT    // spuk per node
        + EDGE_COUNT    // edge closed
        + 1             // spuk count
        + NODE_COUNT    // jewel in node
        + JEWEL_COUNT   // jewel deposited
        + JEWEL_COUNT   // jewel number
        + 1             // next_required_jewel
        + 1             // deck size
        + MAX_FIGURES   // active figure one-hot
        + 6             // phase one-hot
        + 1             // die roll
}

/// Encode a `PlayerView` as a flat `Vec<f32>` for ML consumption.
/// Encoding is deterministic and fixed-size (see `observation_dim()`).
pub fn observation_vector(view: &PlayerView) -> Vec<f32> {
    let mut v = Vec::with_capacity(observation_dim());

    // Figure positions — one-hot per figure over NODE_COUNT nodes.
    for fig in 0..MAX_FIGURES {
        let pos = view.figure_positions[fig] as usize;
        for n in 0..NODE_COUNT {
            v.push(if n == pos { 1.0 } else { 0.0 });
        }
    }

    // Figure carry — index 0 = empty, 1..=JEWEL_COUNT = jewel id+1.
    for fig in 0..MAX_FIGURES {
        let carry = view.figure_carries[fig].map(|j| (j + 1) as usize).unwrap_or(0);
        for c in 0..=(JEWEL_COUNT) {
            v.push(if c == carry { 1.0 } else { 0.0 });
        }
    }

    // Ghosts per node, normalized to [0, 1].
    for ns in &view.node_states {
        v.push(ns.ghosts as f32 / 2.0);
    }

    // Spuk per node.
    for ns in &view.node_states {
        v.push(if ns.has_spuk { 1.0 } else { 0.0 });
    }

    // Edge closed.
    for &closed in &view.edge_closed {
        v.push(if closed { 1.0 } else { 0.0 });
    }

    // Spuk count normalized.
    v.push(view.spuk_count as f32 / 6.0);

    // Jewel in node (any node that could hold a jewel).
    for ns in &view.node_states {
        v.push(if ns.jewel.is_some() { 1.0 } else { 0.0 });
    }

    // Jewel deposited.
    for &dep in &view.jewel_deposited {
        v.push(if dep { 1.0 } else { 0.0 });
    }

    // Jewel number (0 = unknown/not numbered, else (n-1)/7).
    for &num in &view.jewel_number {
        v.push(if num == 0 { 0.0 } else { (num - 1) as f32 / 7.0 });
    }

    // Next required jewel.
    v.push(view.next_required_jewel as f32 / 8.0);

    // Deck size.
    v.push(view.deck_size as f32 / 24.0);

    // Active figure one-hot.
    for f in 0..MAX_FIGURES {
        v.push(if f == view.active_figure as usize { 1.0 } else { 0.0 });
    }

    // Phase one-hot (6 variants of TurnPhase).
    let phase_idx = match view.phase {
        TurnPhase::RollDie => 0,
        TurnPhase::DrawGhostCard { .. } => 1,
        TurnPhase::Move => 2,
        TurnPhase::PickupJewel => 3,
        TurnPhase::Combat => 4,
        TurnPhase::EndTurn => 5,
    };
    for p in 0..6usize {
        v.push(if p == phase_idx { 1.0 } else { 0.0 });
    }

    // Die roll normalized.
    v.push(view.die_roll as f32 / 6.0);

    v
}
