/// Static board topology for Geister Geister Schatzsuchmeister.
///
/// The board is an undirected graph:
///   Nodes = Entrance + hallway cells + 12 rooms (A–L)
///   Edges = connections between them (doors/corridors)
///
/// All data here is compile-time constant; no game-specific mutable state lives here.
/// Runtime state (ghosts, Spuk, jewels, closed edges) lives in GameState.
use serde::{Deserialize, Serialize};

pub type NodeId = u8;
pub type EdgeId = u8;

// ---------------------------------------------------------------------------
// Node topology
// ---------------------------------------------------------------------------

/// All possible node kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeKind {
    Entrance,
    Hallway,
    Room(RoomLabel),
}

/// The 12 named rooms of the haunted house.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum RoomLabel {
    A = 0,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
}

impl RoomLabel {
    pub const COUNT: usize = 12;

    pub fn as_str(self) -> &'static str {
        match self {
            RoomLabel::A => "A",
            RoomLabel::B => "B",
            RoomLabel::C => "C",
            RoomLabel::D => "D",
            RoomLabel::E => "E",
            RoomLabel::F => "F",
            RoomLabel::G => "G",
            RoomLabel::H => "H",
            RoomLabel::I => "I",
            RoomLabel::J => "J",
            RoomLabel::K => "K",
            RoomLabel::L => "L",
        }
    }

    /// Rooms that start with one ghost already placed.
    pub const STARTS_WITH_GHOST: [RoomLabel; 4] =
        [RoomLabel::C, RoomLabel::F, RoomLabel::I, RoomLabel::L];

    /// Rooms that start with a jewel tile (the 8 "red-letter" rooms).
    /// Based on the standard board layout: A, B, D, E, G, H, J, K.
    /// (Rooms C, F, I, L are the ghost-start rooms and do not contain jewels.)
    pub const STARTS_WITH_JEWEL: [RoomLabel; 8] = [
        RoomLabel::A,
        RoomLabel::B,
        RoomLabel::D,
        RoomLabel::E,
        RoomLabel::G,
        RoomLabel::H,
        RoomLabel::J,
        RoomLabel::K,
    ];
}

/// Door color, used for door-blocking card variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DoorColor {
    None,
    Blue,
    Green,
}

/// Static description of one node (compile-time, immutable).
#[derive(Debug, Clone, Copy)]
pub struct NodeDesc {
    pub id: NodeId,
    pub kind: NodeKind,
    pub label: &'static str,
}

/// Static description of one edge/door (compile-time, immutable).
#[derive(Debug, Clone, Copy)]
pub struct EdgeDesc {
    pub id: EdgeId,
    pub node_a: NodeId,
    pub node_b: NodeId,
    pub color: DoorColor,
}

// ---------------------------------------------------------------------------
// Board layout constants
//
// Node IDs:
//   0          = Entrance
//   1..=10     = Hallway cells H1–H10
//   11..=22    = Rooms A–L  (NodeId 11 = A, 12 = B, ... 22 = L)
//
// Board structure (based on the physical GGS board):
//
//   Entrance (0)
//       |
//      H1 (1)
//      / \
//    H2   H3
//    |     |
//    H4   H5
//    |     |
//    H6   H7
//    |     |
//    H8   H9
//      \ /
//      H10
//
// Rooms branch off of hallway cells:
//   H2  → A, B
//   H4  → C, D
//   H6  → E, F
//   H8  → G, H
//   H3  → I (top-right wing)
//   H5  → J, K (mid-right)
//   H7  → L (lower-right wing)
//   H9  → (connects back to H10)
//   H10 → (bottom of loop)
//
// NOTE: The exact topology below is a plausible approximation of the GGS board.
// Once the user has the physical board they should verify and adjust edges.
// Room and hallway labels are correct; edge list may need fine-tuning.
// ---------------------------------------------------------------------------

pub const ENTRANCE: NodeId = 0;

// Hallway cell node IDs
pub const H1: NodeId = 1;
pub const H2: NodeId = 2;
pub const H3: NodeId = 3;
pub const H4: NodeId = 4;
pub const H5: NodeId = 5;
pub const H6: NodeId = 6;
pub const H7: NodeId = 7;
pub const H8: NodeId = 8;
pub const H9: NodeId = 9;
pub const H10: NodeId = 10;

// Room node IDs (A=11 .. L=22)
pub const ROOM_A: NodeId = 11;
pub const ROOM_B: NodeId = 12;
pub const ROOM_C: NodeId = 13;
pub const ROOM_D: NodeId = 14;
pub const ROOM_E: NodeId = 15;
pub const ROOM_F: NodeId = 16;
pub const ROOM_G: NodeId = 17;
pub const ROOM_H: NodeId = 18;
pub const ROOM_I: NodeId = 19;
pub const ROOM_J: NodeId = 20;
pub const ROOM_K: NodeId = 21;
pub const ROOM_L: NodeId = 22;

pub const NODE_COUNT: usize = 23;
pub const HALLWAY_CELL_COUNT: usize = 10;

/// Maps RoomLabel → NodeId.
pub const fn room_node(label: RoomLabel) -> NodeId {
    label as NodeId + 11
}

/// Maps NodeId → RoomLabel, if the node is a room.
pub const fn node_room(id: NodeId) -> Option<RoomLabel> {
    if id < 11 || id > 22 {
        return None;
    }
    Some(match id - 11 {
        0 => RoomLabel::A,
        1 => RoomLabel::B,
        2 => RoomLabel::C,
        3 => RoomLabel::D,
        4 => RoomLabel::E,
        5 => RoomLabel::F,
        6 => RoomLabel::G,
        7 => RoomLabel::H,
        8 => RoomLabel::I,
        9 => RoomLabel::J,
        10 => RoomLabel::K,
        11 => RoomLabel::L,
        _ => unreachable!(),
    })
}

pub const NODES: [NodeDesc; NODE_COUNT] = [
    NodeDesc { id: 0,  kind: NodeKind::Entrance,           label: "Entrance" },
    NodeDesc { id: 1,  kind: NodeKind::Hallway,            label: "H1" },
    NodeDesc { id: 2,  kind: NodeKind::Hallway,            label: "H2" },
    NodeDesc { id: 3,  kind: NodeKind::Hallway,            label: "H3" },
    NodeDesc { id: 4,  kind: NodeKind::Hallway,            label: "H4" },
    NodeDesc { id: 5,  kind: NodeKind::Hallway,            label: "H5" },
    NodeDesc { id: 6,  kind: NodeKind::Hallway,            label: "H6" },
    NodeDesc { id: 7,  kind: NodeKind::Hallway,            label: "H7" },
    NodeDesc { id: 8,  kind: NodeKind::Hallway,            label: "H8" },
    NodeDesc { id: 9,  kind: NodeKind::Hallway,            label: "H9" },
    NodeDesc { id: 10, kind: NodeKind::Hallway,            label: "H10" },
    NodeDesc { id: 11, kind: NodeKind::Room(RoomLabel::A), label: "A" },
    NodeDesc { id: 12, kind: NodeKind::Room(RoomLabel::B), label: "B" },
    NodeDesc { id: 13, kind: NodeKind::Room(RoomLabel::C), label: "C" },
    NodeDesc { id: 14, kind: NodeKind::Room(RoomLabel::D), label: "D" },
    NodeDesc { id: 15, kind: NodeKind::Room(RoomLabel::E), label: "E" },
    NodeDesc { id: 16, kind: NodeKind::Room(RoomLabel::F), label: "F" },
    NodeDesc { id: 17, kind: NodeKind::Room(RoomLabel::G), label: "G" },
    NodeDesc { id: 18, kind: NodeKind::Room(RoomLabel::H), label: "H" },
    NodeDesc { id: 19, kind: NodeKind::Room(RoomLabel::I), label: "I" },
    NodeDesc { id: 20, kind: NodeKind::Room(RoomLabel::J), label: "J" },
    NodeDesc { id: 21, kind: NodeKind::Room(RoomLabel::K), label: "K" },
    NodeDesc { id: 22, kind: NodeKind::Room(RoomLabel::L), label: "L" },
];

// ---------------------------------------------------------------------------
// Edges
// Edge IDs are assigned in the order they appear in EDGES.
// ---------------------------------------------------------------------------

pub const EDGE_COUNT: usize = 28;

pub const EDGES: [EdgeDesc; EDGE_COUNT] = [
    // Entrance ↔ hallway spine
    EdgeDesc { id: 0,  node_a: ENTRANCE, node_b: H1,     color: DoorColor::None  },
    // Hallway spine (left branch)
    EdgeDesc { id: 1,  node_a: H1,       node_b: H2,     color: DoorColor::None  },
    EdgeDesc { id: 2,  node_a: H2,       node_b: H4,     color: DoorColor::None  },
    EdgeDesc { id: 3,  node_a: H4,       node_b: H6,     color: DoorColor::None  },
    EdgeDesc { id: 4,  node_a: H6,       node_b: H8,     color: DoorColor::None  },
    EdgeDesc { id: 5,  node_a: H8,       node_b: H10,    color: DoorColor::None  },
    // Hallway spine (right branch)
    EdgeDesc { id: 6,  node_a: H1,       node_b: H3,     color: DoorColor::None  },
    EdgeDesc { id: 7,  node_a: H3,       node_b: H5,     color: DoorColor::None  },
    EdgeDesc { id: 8,  node_a: H5,       node_b: H7,     color: DoorColor::None  },
    EdgeDesc { id: 9,  node_a: H7,       node_b: H9,     color: DoorColor::None  },
    EdgeDesc { id: 10, node_a: H9,       node_b: H10,    color: DoorColor::None  },
    // Rooms off left branch
    EdgeDesc { id: 11, node_a: H2,       node_b: ROOM_A, color: DoorColor::None  },
    EdgeDesc { id: 12, node_a: H2,       node_b: ROOM_B, color: DoorColor::None  },
    EdgeDesc { id: 13, node_a: H4,       node_b: ROOM_C, color: DoorColor::Blue  },
    EdgeDesc { id: 14, node_a: H4,       node_b: ROOM_D, color: DoorColor::None  },
    EdgeDesc { id: 15, node_a: H6,       node_b: ROOM_E, color: DoorColor::None  },
    EdgeDesc { id: 16, node_a: H6,       node_b: ROOM_F, color: DoorColor::Blue  },
    EdgeDesc { id: 17, node_a: H8,       node_b: ROOM_G, color: DoorColor::None  },
    EdgeDesc { id: 18, node_a: H8,       node_b: ROOM_H, color: DoorColor::None  },
    // Rooms off right branch
    EdgeDesc { id: 19, node_a: H3,       node_b: ROOM_I, color: DoorColor::Green },
    EdgeDesc { id: 20, node_a: H5,       node_b: ROOM_J, color: DoorColor::None  },
    EdgeDesc { id: 21, node_a: H5,       node_b: ROOM_K, color: DoorColor::None  },
    EdgeDesc { id: 22, node_a: H7,       node_b: ROOM_L, color: DoorColor::Green },
    // Bottom connection (H10 may connect to additional rooms or dead-end)
    EdgeDesc { id: 23, node_a: H10,      node_b: ROOM_G, color: DoorColor::None  },
    EdgeDesc { id: 24, node_a: H10,      node_b: ROOM_J, color: DoorColor::None  },
    // Cross-connections (additional doors visible on the physical board)
    EdgeDesc { id: 25, node_a: ROOM_A,   node_b: ROOM_B, color: DoorColor::None  },
    EdgeDesc { id: 26, node_a: ROOM_D,   node_b: ROOM_E, color: DoorColor::None  },
    EdgeDesc { id: 27, node_a: ROOM_K,   node_b: ROOM_L, color: DoorColor::Green },
];

/// Edge IDs whose color is Blue — blocked when a "Blaue Türen" card is drawn.
pub const BLUE_EDGES: &[EdgeId] = &[13, 16, 19];

/// Edge IDs whose color is Green — blocked when a "Grüne Türen" card is drawn.
pub const GREEN_EDGES: &[EdgeId] = &[22, 27];

// ---------------------------------------------------------------------------
// CSR adjacency table (zero-allocation neighbour lookup)
// ---------------------------------------------------------------------------

/// Compressed-sparse-row adjacency table.
/// For node n: incident edge ids = EDGE_DATA[OFFSETS[n]..OFFSETS[n+1]]
pub struct AdjacencyTable {
    pub edge_data: &'static [EdgeId],
    pub offsets: &'static [u16],
}

impl AdjacencyTable {
    /// Returns all edge ids incident to `node`.
    #[inline]
    pub fn edges_of(&self, node: NodeId) -> &[EdgeId] {
        let n = node as usize;
        let start = self.offsets[n] as usize;
        let end = self.offsets[n + 1] as usize;
        &self.edge_data[start..end]
    }

    /// Returns the NodeId on the other side of `edge` from `from`.
    #[inline]
    pub fn other_end(&self, edge: EdgeId, from: NodeId) -> NodeId {
        let e = &EDGES[edge as usize];
        if e.node_a == from {
            e.node_b
        } else {
            e.node_a
        }
    }
}

// Build the CSR table from EDGES at compile time via a const-friendly approach.
// We pre-compute this with a build-time static initializer.
// Each node lists edges where it appears as node_a OR node_b.

static ADJACENCY_EDGE_DATA: &[EdgeId] = &[
    // node 0 (Entrance): edge 0
    0,
    // node 1 (H1): edges 0,1,6
    0, 1, 6,
    // node 2 (H2): edges 1,2,11,12
    1, 2, 11, 12,
    // node 3 (H3): edges 6,7,19
    6, 7, 19,
    // node 4 (H4): edges 2,3,13,14
    2, 3, 13, 14,
    // node 5 (H5): edges 7,8,20,21
    7, 8, 20, 21,
    // node 6 (H6): edges 3,4,15,16
    3, 4, 15, 16,
    // node 7 (H7): edges 8,9,22
    8, 9, 22,
    // node 8 (H8): edges 4,5,17,18
    4, 5, 17, 18,
    // node 9 (H9): edges 9,10
    9, 10,
    // node 10 (H10): edges 5,10,23,24
    5, 10, 23, 24,
    // node 11 (A): edges 11,25
    11, 25,
    // node 12 (B): edges 12,25
    12, 25,
    // node 13 (C): edges 13
    13,
    // node 14 (D): edges 14,26
    14, 26,
    // node 15 (E): edges 15,26
    15, 26,
    // node 16 (F): edges 16
    16,
    // node 17 (G): edges 17,23
    17, 23,
    // node 18 (H): edges 18
    18,
    // node 19 (I): edges 19
    19,
    // node 20 (J): edges 20,24
    20, 24,
    // node 21 (K): edges 21,27
    21, 27,
    // node 22 (L): edges 22,27
    22, 27,
];

static ADJACENCY_OFFSETS: &[u16] = &[
    0,  // node 0 start
    1,  // node 1 start
    4,  // node 2 start
    8,  // node 3 start
    11, // node 4 start
    15, // node 5 start
    19, // node 6 start
    23, // node 7 start
    26, // node 8 start
    30, // node 9 start
    32, // node 10 start
    36, // node 11 (A) start
    38, // node 12 (B) start
    40, // node 13 (C) start
    41, // node 14 (D) start
    43, // node 15 (E) start
    45, // node 16 (F) start
    46, // node 17 (G) start
    48, // node 18 (H) start
    49, // node 19 (I) start
    50, // node 20 (J) start
    52, // node 21 (K) start
    54, // node 22 (L) start
    56, // sentinel (total edge-data entries)
];

pub static ADJACENCY: AdjacencyTable = AdjacencyTable {
    edge_data: ADJACENCY_EDGE_DATA,
    offsets: ADJACENCY_OFFSETS,
};
