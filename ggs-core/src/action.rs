use crate::board::EdgeId;
use crate::state::JewelId;
use serde::{Deserialize, Serialize};

/// Every action a figure can take on its turn.
/// The engine validates and resolves all actions; strategies pick from `legal_actions`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Action {
    /// Phase: RollDie. The engine always rolls internally; strategies never produce this.
    /// Kept in the enum so replay trajectories are complete.
    RollDie,

    /// Phase: DrawGhostCard. Engine draws and resolves automatically.
    DrawGhostCard,

    /// Phase: Move. Move the active figure along the given edge to the adjacent node.
    MoveAlongEdge { edge: EdgeId },

    /// Phase: Move. Stop moving and use remaining movement points as 0.
    StopMoving,

    /// Phase: PickupJewel. Pick up the specified jewel in the current room.
    PickupJewel { jewel: JewelId },

    /// Phase: PickupJewel. Skip jewel pickup.
    SkipPickup,

    /// Phase: PickupJewel or EndTurn. Deposit the carried jewel at the entrance.
    /// Legal when the figure is at the Entrance node and carrying a jewel.
    DepositJewel,

    /// Phase: Combat. Roll symbol dice to fight ghosts/Spuk in the current room.
    Fight,

    /// Phase: Combat. Skip combat and end the turn.
    SkipCombat,

    /// Phase: EndTurn. Advance to the next figure (auto-resolved by engine).
    EndTurn,
}

/// Face values on the two symbol dice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolFace {
    Ghost,
    Spuk,
    Blank,
}

impl SymbolFace {
    /// The six faces of one symbol die.
    pub const FACES: [SymbolFace; 6] = [
        SymbolFace::Ghost,
        SymbolFace::Ghost,
        SymbolFace::Ghost,
        SymbolFace::Blank,
        SymbolFace::Spuk,
        SymbolFace::Spuk,
    ];
}

/// Outcome of rolling both symbol dice (used when 2 dice are rolled).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CombatRoll {
    pub die_a: SymbolFace,
    pub die_b: SymbolFace,
}

impl CombatRoll {
    pub fn has_spuk(self) -> bool {
        self.die_a == SymbolFace::Spuk || self.die_b == SymbolFace::Spuk
    }
    pub fn ghost_count(self) -> u8 {
        (self.die_a == SymbolFace::Ghost) as u8 + (self.die_b == SymbolFace::Ghost) as u8
    }
}
