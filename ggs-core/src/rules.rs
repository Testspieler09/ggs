/// Pure query functions over `&GameState`.
/// No mutation. The engine calls these; strategies may call them via `PlayerView`.
use crate::action::Action;
use crate::board::{EdgeId, NodeId, NodeKind, ADJACENCY, ENTRANCE, NODE_COUNT};
use crate::state::{GameState, JewelId, TurnPhase};

// ---------------------------------------------------------------------------
// Terminal conditions
// ---------------------------------------------------------------------------

pub fn is_win(state: &GameState) -> bool {
    if !state.jewel_deposited.iter().all(|&d| d) {
        return false;
    }
    // All active figures must be at the entrance.
    (0..state.figure_count as usize).all(|i| state.figure_pos[i] == crate::board::ENTRANCE)
}

pub fn is_loss(state: &GameState) -> bool {
    state.spuk_count >= crate::state::MAX_SPUK
}

pub fn is_terminal(state: &GameState) -> bool {
    state.game_over
}

// ---------------------------------------------------------------------------
// Edge / movement helpers
// ---------------------------------------------------------------------------

/// True if the given edge is currently passable (not blocked).
#[inline]
pub fn edge_passable(state: &GameState, edge: EdgeId) -> bool {
    !state.edge_states[edge as usize].closed
}

/// For a figure at `from_node`, returns the node on the other side of `edge`.
#[inline]
pub fn other_end(edge: EdgeId, from: NodeId) -> NodeId {
    ADJACENCY.other_end(edge, from)
}

/// Returns all edges the active figure can traverse in one step from their current node.
/// Filters out blocked edges and hallway cells already occupied by another figure.
pub fn reachable_edges(state: &GameState) -> impl Iterator<Item = EdgeId> + '_ {
    let fig = state.active_figure;
    let pos = state.figure_pos[fig as usize];
    ADJACENCY.edges_of(pos).iter().copied().filter(move |&e| {
        if !edge_passable(state, e) {
            return false;
        }
        let dest = other_end(e, pos);
        // Hallway cells: at most one figure.
        if matches!(crate::board::NODES[dest as usize].kind, NodeKind::Hallway) {
            state.node_states[dest as usize].figure_count() == 0
        } else {
            true
        }
    })
}

// ---------------------------------------------------------------------------
// Jewel helpers
// ---------------------------------------------------------------------------

/// Returns the jewel id currently in `node`, if any uncollected jewel is present.
pub fn jewel_in_node(state: &GameState, node: NodeId) -> Option<JewelId> {
    state.node_states[node as usize].jewel
}

/// True if the active figure may pick up a jewel right now.
/// Conditions:
///   - Figure is in a Room node (not hallway/entrance)
///   - A jewel is present in that room
///   - Figure is not already carrying a jewel
///   - Numbered-jewel variant: the jewel's revealed number matches `next_required_jewel`
pub fn can_pickup(state: &GameState) -> bool {
    let fig = state.active_figure as usize;
    if state.figure_carries[fig].is_some() {
        return false;
    }
    let pos = state.figure_pos[fig];
    if !matches!(crate::board::NODES[pos as usize].kind, NodeKind::Room(_)) {
        return false;
    }
    let Some(jewel_id) = state.node_states[pos as usize].jewel else {
        return false;
    };
    if state.variant.numbered_jewels {
        let revealed = state.jewel_number[jewel_id as usize];
        // revealed == 0 means not yet revealed (figure hasn't entered the room before)
        // The engine reveals the number when the figure enters; if still 0 here, it means
        // the engine hasn't run the reveal yet — treat as not-pickable until revealed.
        revealed != 0 && revealed == state.next_required_jewel
    } else {
        true
    }
}

/// True if there are ghosts or Spuk in the active figure's room that can be fought.
pub fn can_fight(state: &GameState) -> bool {
    let pos = state.figure_pos[state.active_figure as usize];
    if !matches!(crate::board::NODES[pos as usize].kind, NodeKind::Room(_)) {
        return false;
    }
    let ns = &state.node_states[pos as usize];
    ns.ghosts > 0 || ns.has_spuk
}

/// True if a Spuk in the active figure's room can be fought (requires 2+ figures in room).
pub fn can_fight_spuk(state: &GameState) -> bool {
    let pos = state.figure_pos[state.active_figure as usize];
    let ns = &state.node_states[pos as usize];
    ns.has_spuk && ns.figure_count() >= 2
}

// ---------------------------------------------------------------------------
// Legal action computation
// ---------------------------------------------------------------------------

/// Writes the legal actions for the current state into `buf`.
/// Zero-allocation in the inner simulation loop when the caller reuses `buf`.
pub fn legal_actions_into(state: &GameState, buf: &mut Vec<Action>) {
    buf.clear();
    if state.game_over {
        return;
    }
    match state.phase {
        TurnPhase::RollDie => {
            buf.push(Action::RollDie);
        }
        TurnPhase::DrawGhostCard { .. } => {
            buf.push(Action::DrawGhostCard);
        }
        TurnPhase::Move => {
            for edge in reachable_edges(state) {
                buf.push(Action::MoveAlongEdge { edge });
            }
            buf.push(Action::StopMoving);
            // Deposit jewel at entrance if carrying one and currently at entrance.
            let fig = state.active_figure as usize;
            if state.figure_carries[fig].is_some()
                && state.figure_pos[fig] == ENTRANCE
            {
                buf.push(Action::DepositJewel);
            }
        }
        TurnPhase::PickupJewel => {
            // Deposit first if at entrance with a jewel (can happen if movement ended there).
            let fig = state.active_figure as usize;
            if state.figure_carries[fig].is_some()
                && state.figure_pos[fig] == ENTRANCE
            {
                buf.push(Action::DepositJewel);
            } else if can_pickup(state) {
                let pos = state.figure_pos[fig];
                if let Some(jewel) = state.node_states[pos as usize].jewel {
                    buf.push(Action::PickupJewel { jewel });
                }
            }
            buf.push(Action::SkipPickup);
        }
        TurnPhase::Combat => {
            if can_fight(state) {
                buf.push(Action::Fight);
            }
            buf.push(Action::SkipCombat);
        }
        TurnPhase::EndTurn => {
            buf.push(Action::EndTurn);
        }
    }
}

/// Heap-allocating convenience wrapper around `legal_actions_into`.
pub fn legal_actions(state: &GameState) -> Vec<Action> {
    let mut buf = Vec::with_capacity(8);
    legal_actions_into(state, &mut buf);
    buf
}

// ---------------------------------------------------------------------------
// BFS reachability (used by greedy strategy and MCTS)
// ---------------------------------------------------------------------------

/// BFS distance from `start` to the nearest node satisfying `goal`.
/// Returns `None` if unreachable. Respects closed edges and hallway occupancy.
pub fn bfs_distance<F>(state: &GameState, start: NodeId, goal: F) -> Option<u32>
where
    F: Fn(NodeId) -> bool,
{
    if goal(start) {
        return Some(0);
    }
    let mut visited = [false; NODE_COUNT];
    visited[start as usize] = true;
    let mut queue: std::collections::VecDeque<(NodeId, u32)> = std::collections::VecDeque::new();
    queue.push_back((start, 0));
    while let Some((node, dist)) = queue.pop_front() {
        for &edge in ADJACENCY.edges_of(node) {
            if !edge_passable(state, edge) {
                continue;
            }
            let next = ADJACENCY.other_end(edge, node);
            if visited[next as usize] {
                continue;
            }
            // Hallway occupancy check (figures other than the active one block the cell).
            if matches!(crate::board::NODES[next as usize].kind, NodeKind::Hallway) {
                let ns = &state.node_states[next as usize];
                // Occupied by someone other than the active figure.
                let others = ns.figures & !(1 << state.active_figure);
                if others != 0 {
                    continue;
                }
            }
            visited[next as usize] = true;
            if goal(next) {
                return Some(dist + 1);
            }
            queue.push_back((next, dist + 1));
        }
    }
    None
}
