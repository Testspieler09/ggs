use crate::board::{NodeId, RoomLabel, EDGE_COUNT, NODE_COUNT};
use crate::variant::Variant;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

pub type FigureId = u8;
pub type JewelId = u8;

pub const MAX_FIGURES: usize = 4;
pub const JEWEL_COUNT: usize = 8;
pub const GHOST_DECK_CAPACITY: usize = 19;
pub const MAX_SPUK: u8 = 6;
pub const MAX_GHOSTS_BEFORE_SPUK: u8 = 3;

// ---------------------------------------------------------------------------
// Per-node and per-edge runtime state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct NodeState {
    /// Ghost figures present (0–2; placing a 3rd converts to Spuk).
    pub ghosts: u8,
    /// Whether a Spuk (haunt) figure occupies this room.
    pub has_spuk: bool,
    /// Which jewel is in this node, if any.
    pub jewel: Option<JewelId>,
    /// Pre-assigned number for the jewel in this room (1–8); 0 = no jewel or base variant.
    /// Hidden from strategies until revealed; the engine copies this into GameState::jewel_number
    /// when a figure first enters the room.
    pub(crate) jewel_number: u8,
    /// Bitmask of figures currently here (bit i = FigureId i).
    pub figures: u8,
}

impl NodeState {
    pub fn has_figure(&self, fig: FigureId) -> bool {
        self.figures & (1 << fig) != 0
    }
    pub fn add_figure(&mut self, fig: FigureId) {
        self.figures |= 1 << fig;
    }
    pub fn remove_figure(&mut self, fig: FigureId) {
        self.figures &= !(1 << fig);
    }
    pub fn figure_count(&self) -> u8 {
        self.figures.count_ones() as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct EdgeState {
    /// Whether this door/passage is currently blocked.
    pub closed: bool,
}

// ---------------------------------------------------------------------------
// Ghost card deck
// ---------------------------------------------------------------------------

/// A ghost card identifies the room where a new ghost is placed,
/// or carries a special action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GhostCard {
    /// Place one ghost in the named room.
    Room(RoomLabel),
    /// Draw 2 additional ghost cards (advanced variant).
    DrawTwo,
    /// Draw 3 additional ghost cards (advanced variant).
    DrawThree,
    /// Reshuffle the entire deck immediately.
    Reshuffle,
    /// Block all blue-colored doors (advanced variant).
    BlueDoors,
    /// Block all green-colored doors (advanced variant).
    GreenDoors,
}

/// Inline deck; no heap. The cursor `top` points to the next undrawn card.
/// Cards 0..top have already been drawn (discard pile equivalent).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GhostDeck {
    pub cards: [GhostCard; GHOST_DECK_CAPACITY],
    /// Index of the next card to draw.
    pub top: u8,
    /// Number of cards currently in the deck (changes on reshuffle).
    pub size: u8,
}

impl GhostDeck {
    pub fn is_empty(&self) -> bool {
        self.top >= self.size
    }

    pub fn remaining(&self) -> u8 {
        self.size.saturating_sub(self.top)
    }

    /// Draw the next card and advance the cursor.
    /// Returns `None` if the deck is empty (engine must reshuffle first).
    pub fn draw(&mut self) -> Option<GhostCard> {
        if self.is_empty() {
            return None;
        }
        let card = self.cards[self.top as usize];
        self.top += 1;
        Some(card)
    }

    /// Reset so all cards are drawable again (used after Reshuffle card).
    pub fn reset_cursor(&mut self) {
        self.top = 0;
    }
}

// ---------------------------------------------------------------------------
// Turn phase state machine
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TurnPhase {
    /// Active figure must roll the number die.
    RollDie,
    /// Active figure must draw (and resolve) ghost card(s).
    /// Only reached when the die roll is < 6; a roll of 6 skips straight to Move.
    DrawGhostCard { cards_remaining: u8 },
    /// Active figure may move (up to `moves_remaining` steps).
    Move,
    /// Active figure may pick up a jewel in their current room (always optional).
    PickupJewel,
    /// Active figure may fight ghosts/Spuk in their current room.
    Combat,
    /// Turn is complete; engine will advance to the next figure.
    EndTurn,
}

// ---------------------------------------------------------------------------
// Full game state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GameState {
    // --- Configuration (set once at setup, never mutated) ---
    pub variant: Variant,
    /// Number of active figures (3 or 4).
    pub figure_count: u8,

    // --- Board graph runtime state ---
    #[serde(with = "BigArray")]
    pub node_states: [NodeState; NODE_COUNT],
    #[serde(with = "BigArray")]
    pub edge_states: [EdgeState; EDGE_COUNT],

    // --- Figure positions and inventory ---
    /// Current node for each figure.
    pub figure_pos: [NodeId; MAX_FIGURES],
    /// Jewel carried by each figure, if any.
    pub figure_carries: [Option<JewelId>; MAX_FIGURES],

    // NOTE: the `jewel_deposited` and `jewel_number` fields could be optimized
    // by using a u8 to encode the information

    // --- Jewel state ---
    /// Whether each jewel has been safely deposited at the entrance.
    pub jewel_deposited: [bool; JEWEL_COUNT],
    /// Revealed number (1–8) for each jewel; 0 = not yet revealed.
    /// Set when a figure first enters the room containing that jewel.
    pub jewel_number: [u8; JEWEL_COUNT],
    /// Next jewel number that must be collected (numbered-jewel variant).
    pub next_required_jewel: u8,

    // --- Haunt tracker ---
    /// Total Spuk figures currently on the board. Game over when this reaches 6.
    pub spuk_count: u8,

    // --- Card deck ---
    pub deck: GhostDeck,

    // --- Turn tracking ---
    pub active_figure: FigureId,
    pub phase: TurnPhase,
    pub die_roll: u8,
    pub moves_remaining: u8,

    // --- Terminal flags ---
    pub game_over: bool,
    pub players_won: bool,
}
