use serde::{Deserialize, Serialize};

/// Variant/difficulty configuration for one simulation run.
/// All flags are independent — any combination is valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Variant {
    /// Include the "Zieh 2" card: draw 2 ghost cards on that trigger.
    pub draw_two_card: bool,
    /// Include the "Zieh 3" card: draw 3 ghost cards on that trigger.
    pub draw_three_card: bool,
    /// Include blue/green door-blocking cards.
    pub door_cards: bool,
    /// Jewels are face-down 1–8; must be collected in ascending order.
    pub numbered_jewels: bool,
}

impl Variant {
    pub const BASE: Self = Self {
        draw_two_card: false,
        draw_three_card: false,
        door_cards: false,
        numbered_jewels: false,
    };

    pub const ADVANCED: Self = Self {
        draw_two_card: true,
        draw_three_card: true,
        door_cards: true,
        numbered_jewels: true,
    };
}

impl Default for Variant {
    fn default() -> Self {
        Self::BASE
    }
}
