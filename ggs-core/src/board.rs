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
        RoomLabel::C,
        RoomLabel::D,
        RoomLabel::E,
        RoomLabel::G,
        RoomLabel::I,
        RoomLabel::J,
        RoomLabel::L,
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
pub const H11: NodeId = 11;
pub const H12: NodeId = 12;
pub const H13: NodeId = 13;
pub const H14: NodeId = 14;
pub const H15: NodeId = 15;
pub const H16: NodeId = 16;
pub const H17: NodeId = 17;
pub const H18: NodeId = 18;
pub const H19: NodeId = 19;
pub const H20: NodeId = 20;
pub const H21: NodeId = 21;
pub const H22: NodeId = 22;
pub const H23: NodeId = 23;

// Room node IDs (A=24 .. L=35)
pub const ROOM_A: NodeId = 24;
pub const ROOM_B: NodeId = 25;
pub const ROOM_C: NodeId = 26;
pub const ROOM_D: NodeId = 27;
pub const ROOM_E: NodeId = 28;
pub const ROOM_F: NodeId = 29;
pub const ROOM_G: NodeId = 30;
pub const ROOM_H: NodeId = 31;
pub const ROOM_I: NodeId = 32;
pub const ROOM_J: NodeId = 33;
pub const ROOM_K: NodeId = 34;
pub const ROOM_L: NodeId = 35;

pub const NODE_COUNT: usize = 36;
pub const HALLWAY_CELL_COUNT: usize = 23;

/// Maps RoomLabel → NodeId. Rooms start at 24 (after Entrance + H1–H23).
pub const fn room_node(label: RoomLabel) -> NodeId {
    label as NodeId + 24
}

/// Maps NodeId → RoomLabel, if the node is a room.
pub const fn node_room(id: NodeId) -> Option<RoomLabel> {
    if id < 24 || id > 35 {
        return None;
    }
    Some(match id - 24 {
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
    NodeDesc {
        id: 0,
        kind: NodeKind::Entrance,
        label: "Entrance",
    },
    NodeDesc {
        id: 1,
        kind: NodeKind::Hallway,
        label: "H1",
    },
    NodeDesc {
        id: 2,
        kind: NodeKind::Hallway,
        label: "H2",
    },
    NodeDesc {
        id: 3,
        kind: NodeKind::Hallway,
        label: "H3",
    },
    NodeDesc {
        id: 4,
        kind: NodeKind::Hallway,
        label: "H4",
    },
    NodeDesc {
        id: 5,
        kind: NodeKind::Hallway,
        label: "H5",
    },
    NodeDesc {
        id: 6,
        kind: NodeKind::Hallway,
        label: "H6",
    },
    NodeDesc {
        id: 7,
        kind: NodeKind::Hallway,
        label: "H7",
    },
    NodeDesc {
        id: 8,
        kind: NodeKind::Hallway,
        label: "H8",
    },
    NodeDesc {
        id: 9,
        kind: NodeKind::Hallway,
        label: "H9",
    },
    NodeDesc {
        id: 10,
        kind: NodeKind::Hallway,
        label: "H10",
    },
    NodeDesc {
        id: 11,
        kind: NodeKind::Hallway,
        label: "H11",
    },
    NodeDesc {
        id: 12,
        kind: NodeKind::Hallway,
        label: "H12",
    },
    NodeDesc {
        id: 13,
        kind: NodeKind::Hallway,
        label: "H13",
    },
    NodeDesc {
        id: 14,
        kind: NodeKind::Hallway,
        label: "H14",
    },
    NodeDesc {
        id: 15,
        kind: NodeKind::Hallway,
        label: "H15",
    },
    NodeDesc {
        id: 16,
        kind: NodeKind::Hallway,
        label: "H16",
    },
    NodeDesc {
        id: 17,
        kind: NodeKind::Hallway,
        label: "H17",
    },
    NodeDesc {
        id: 18,
        kind: NodeKind::Hallway,
        label: "H18",
    },
    NodeDesc {
        id: 19,
        kind: NodeKind::Hallway,
        label: "H19",
    },
    NodeDesc {
        id: 20,
        kind: NodeKind::Hallway,
        label: "H20",
    },
    NodeDesc {
        id: 21,
        kind: NodeKind::Hallway,
        label: "H21",
    },
    NodeDesc {
        id: 22,
        kind: NodeKind::Hallway,
        label: "H22",
    },
    NodeDesc {
        id: 23,
        kind: NodeKind::Hallway,
        label: "H23",
    },
    NodeDesc {
        id: 24,
        kind: NodeKind::Room(RoomLabel::A),
        label: "A",
    },
    NodeDesc {
        id: 25,
        kind: NodeKind::Room(RoomLabel::B),
        label: "B",
    },
    NodeDesc {
        id: 26,
        kind: NodeKind::Room(RoomLabel::C),
        label: "C",
    },
    NodeDesc {
        id: 27,
        kind: NodeKind::Room(RoomLabel::D),
        label: "D",
    },
    NodeDesc {
        id: 28,
        kind: NodeKind::Room(RoomLabel::E),
        label: "E",
    },
    NodeDesc {
        id: 29,
        kind: NodeKind::Room(RoomLabel::F),
        label: "F",
    },
    NodeDesc {
        id: 30,
        kind: NodeKind::Room(RoomLabel::G),
        label: "G",
    },
    NodeDesc {
        id: 31,
        kind: NodeKind::Room(RoomLabel::H),
        label: "H",
    },
    NodeDesc {
        id: 32,
        kind: NodeKind::Room(RoomLabel::I),
        label: "I",
    },
    NodeDesc {
        id: 33,
        kind: NodeKind::Room(RoomLabel::J),
        label: "J",
    },
    NodeDesc {
        id: 34,
        kind: NodeKind::Room(RoomLabel::K),
        label: "K",
    },
    NodeDesc {
        id: 35,
        kind: NodeKind::Room(RoomLabel::L),
        label: "L",
    },
];

// ---------------------------------------------------------------------------
// Edges
// Edge IDs are assigned in the order they appear in EDGES.
// ---------------------------------------------------------------------------

pub const EDGE_COUNT: usize = 45;

pub const EDGES: [EdgeDesc; EDGE_COUNT] = [
    // All hallway edges including to rooms
    EdgeDesc {
        id: 0,
        node_a: ENTRANCE,
        node_b: H1,
        color: DoorColor::None,
    },
    EdgeDesc {
        id: 1,
        node_a: H1,
        node_b: H2,
        color: DoorColor::None,
    },
    EdgeDesc {
        id: 2,
        node_a: H2,
        node_b: H3,
        color: DoorColor::None,
    },
    EdgeDesc {
        id: 3,
        node_a: H2,
        node_b: ROOM_J,
        color: DoorColor::Blue,
    },
    EdgeDesc {
        id: 4,
        node_a: H2,
        node_b: ROOM_L,
        color: DoorColor::Green,
    },
    EdgeDesc {
        id: 5,
        node_a: H3,
        node_b: H4,
        color: DoorColor::None,
    },
    EdgeDesc {
        id: 6,
        node_a: H3,
        node_b: H10,
        color: DoorColor::None,
    },
    EdgeDesc {
        id: 7,
        node_a: H4,
        node_b: H5,
        color: DoorColor::None,
    },
    EdgeDesc {
        id: 8,
        node_a: H5,
        node_b: H6,
        color: DoorColor::None,
    },
    EdgeDesc {
        id: 9,
        node_a: H5,
        node_b: ROOM_I,
        color: DoorColor::Blue,
    },
    EdgeDesc {
        id: 10,
        node_a: H6,
        node_b: H7,
        color: DoorColor::None,
    },
    EdgeDesc {
        id: 11,
        node_a: H7,
        node_b: H8,
        color: DoorColor::None,
    },
    EdgeDesc {
        id: 12,
        node_a: H8,
        node_b: H9,
        color: DoorColor::None,
    },
    EdgeDesc {
        id: 13,
        node_a: H8,
        node_b: ROOM_H,
        color: DoorColor::Green,
    },
    EdgeDesc {
        id: 14,
        node_a: H9,
        node_b: ROOM_K,
        color: DoorColor::Blue,
    },
    EdgeDesc {
        id: 15,
        node_a: H10,
        node_b: H11,
        color: DoorColor::None,
    },
    EdgeDesc {
        id: 16,
        node_a: H11,
        node_b: H12,
        color: DoorColor::None,
    },
    EdgeDesc {
        id: 17,
        node_a: H12,
        node_b: H13,
        color: DoorColor::None,
    },
    EdgeDesc {
        id: 18,
        node_a: H12,
        node_b: ROOM_F,
        color: DoorColor::Blue,
    },
    EdgeDesc {
        id: 19,
        node_a: H13,
        node_b: H16,
        color: DoorColor::None,
    },
    EdgeDesc {
        id: 20,
        node_a: H13,
        node_b: ROOM_G,
        color: DoorColor::Green,
    },
    EdgeDesc {
        id: 21,
        node_a: H16,
        node_b: H17,
        color: DoorColor::None,
    },
    EdgeDesc {
        id: 22,
        node_a: H17,
        node_b: H18,
        color: DoorColor::None,
    },
    EdgeDesc {
        id: 23,
        node_a: H18,
        node_b: H19,
        color: DoorColor::None,
    },
    EdgeDesc {
        id: 24,
        node_a: H18,
        node_b: ROOM_F,
        color: DoorColor::Green,
    },
    EdgeDesc {
        id: 25,
        node_a: H19,
        node_b: H20,
        color: DoorColor::None,
    },
    EdgeDesc {
        id: 26,
        node_a: H19,
        node_b: ROOM_B,
        color: DoorColor::Blue,
    },
    EdgeDesc {
        id: 27,
        node_a: H20,
        node_b: H21,
        color: DoorColor::None,
    },
    EdgeDesc {
        id: 28,
        node_a: H21,
        node_b: H22,
        color: DoorColor::None,
    },
    EdgeDesc {
        id: 29,
        node_a: H22,
        node_b: H23,
        color: DoorColor::None,
    },
    EdgeDesc {
        id: 30,
        node_a: H22,
        node_b: ROOM_A,
        color: DoorColor::Green,
    },
    EdgeDesc {
        id: 31,
        node_a: H22,
        node_b: ROOM_E,
        color: DoorColor::Blue,
    },
    EdgeDesc {
        id: 32,
        node_a: H16,
        node_b: H15,
        color: DoorColor::None,
    },
    EdgeDesc {
        id: 33,
        node_a: H15,
        node_b: H14,
        color: DoorColor::None,
    },
    EdgeDesc {
        id: 34,
        node_a: H15,
        node_b: ROOM_C,
        color: DoorColor::Green,
    },
    EdgeDesc {
        id: 35,
        node_a: H14,
        node_b: ROOM_D,
        color: DoorColor::Blue,
    },
    // All room edges
    // Top isle
    EdgeDesc {
        id: 36,
        node_a: ROOM_A,
        node_b: ROOM_B,
        color: DoorColor::None,
    },
    EdgeDesc {
        id: 37,
        node_a: ROOM_B,
        node_b: ROOM_C,
        color: DoorColor::None,
    },
    EdgeDesc {
        id: 38,
        node_a: ROOM_C,
        node_b: ROOM_D,
        color: DoorColor::None,
    },
    // Middle-left isle
    EdgeDesc {
        id: 39,
        node_a: ROOM_E,
        node_b: ROOM_F,
        color: DoorColor::None,
    },
    EdgeDesc {
        id: 40,
        node_a: ROOM_F,
        node_b: ROOM_I,
        color: DoorColor::None,
    },
    EdgeDesc {
        id: 41,
        node_a: ROOM_I,
        node_b: ROOM_H,
        color: DoorColor::None,
    },
    EdgeDesc {
        id: 42,
        node_a: ROOM_H,
        node_b: ROOM_E,
        color: DoorColor::None,
    },
    // Middle-right isle
    EdgeDesc {
        id: 43,
        node_a: ROOM_G,
        node_b: ROOM_J,
        color: DoorColor::None,
    },
    // Bottom isle
    EdgeDesc {
        id: 44,
        node_a: ROOM_K,
        node_b: ROOM_L,
        color: DoorColor::None,
    },
];

const fn collect_edges_by_color(color: DoorColor) -> ([EdgeId; EDGE_COUNT], usize) {
    let mut out = [0u8; EDGE_COUNT];
    let mut len = 0usize;
    let mut i = 0usize;
    while i < EDGE_COUNT {
        // DoorColor doesn't impl PartialEq in const context, so match explicitly.
        let matches = match color {
            DoorColor::Blue => matches!(EDGES[i].color, DoorColor::Blue),
            DoorColor::Green => matches!(EDGES[i].color, DoorColor::Green),
            DoorColor::None => matches!(EDGES[i].color, DoorColor::None),
        };
        if matches {
            out[len] = EDGES[i].id;
            len += 1;
        }
        i += 1;
    }
    (out, len)
}

const BLUE_EDGES_DATA: ([EdgeId; EDGE_COUNT], usize) = collect_edges_by_color(DoorColor::Blue);
const GREEN_EDGES_DATA: ([EdgeId; EDGE_COUNT], usize) = collect_edges_by_color(DoorColor::Green);

/// Edge IDs whose color is Blue — blocked when a "Blaue Türen" card is drawn.
pub const BLUE_EDGES: &[EdgeId] = BLUE_EDGES_DATA.0.split_at(BLUE_EDGES_DATA.1).0;

/// Edge IDs whose color is Green — blocked when a "Grüne Türen" card is drawn.
pub const GREEN_EDGES: &[EdgeId] = GREEN_EDGES_DATA.0.split_at(GREEN_EDGES_DATA.1).0;

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

struct CsrData {
    edge_data: [EdgeId; EDGE_COUNT * 2],
    offsets: [u16; NODE_COUNT + 1],
}

const fn build_adjacency() -> CsrData {
    // Pass 1: count incident edges per node.
    let mut degree = [0u16; NODE_COUNT];
    let mut i = 0usize;
    while i < EDGE_COUNT {
        degree[EDGES[i].node_a as usize] += 1;
        degree[EDGES[i].node_b as usize] += 1;
        i += 1;
    }

    // Prefix-sum → offsets (offsets[n] = start index for node n).
    let mut offsets = [0u16; NODE_COUNT + 1];
    let mut n = 0usize;
    while n < NODE_COUNT {
        offsets[n + 1] = offsets[n] + degree[n];
        n += 1;
    }

    // Pass 2: fill edge_data; use a cursor array to track insertion position.
    let mut edge_data = [0u8; EDGE_COUNT * 2];
    let mut cursor = [0u16; NODE_COUNT];
    let mut n2 = 0usize;
    while n2 < NODE_COUNT {
        cursor[n2] = offsets[n2];
        n2 += 1;
    }
    let mut i2 = 0usize;
    while i2 < EDGE_COUNT {
        let a = EDGES[i2].node_a as usize;
        let b = EDGES[i2].node_b as usize;
        edge_data[cursor[a] as usize] = EDGES[i2].id;
        cursor[a] += 1;
        edge_data[cursor[b] as usize] = EDGES[i2].id;
        cursor[b] += 1;
        i2 += 1;
    }

    CsrData { edge_data, offsets }
}

static CSR: CsrData = build_adjacency();

static ADJACENCY_EDGE_DATA: &[EdgeId] = &CSR.edge_data;
static ADJACENCY_OFFSETS: &[u16] = &CSR.offsets;

pub static ADJACENCY: AdjacencyTable = AdjacencyTable {
    edge_data: ADJACENCY_EDGE_DATA,
    offsets: ADJACENCY_OFFSETS,
};
