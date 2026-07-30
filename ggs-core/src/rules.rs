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
    if state.spuk_count >= crate::state::MAX_SPUK {
        return true;
    }
    // All figures are carrying a jewel and each is alone in a Spuk room: unwinnable deadlock.
    (0..state.figure_count as usize).all(|i| {
        let pos = state.figure_pos[i];
        state.figure_carries[i].is_some()
            && state.node_states[pos as usize].has_spuk
            && state.node_states[pos as usize].figure_count() == 1
    })
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

/// Returns all edges the active figure can legally traverse in one step from their current node.
///
/// Hallway pass-through rule: a figure may enter an occupied hallway cell (costs 1 move) but
/// cannot stop there. The engine enforces no-stop via suppressing StopMoving in that case.
///
/// Spuk-trap rule: a figure carrying a jewel may not leave a Spuk room.
pub fn reachable_edges(state: &GameState) -> impl Iterator<Item = EdgeId> + '_ {
    let fig = state.active_figure;
    let pos = state.figure_pos[fig as usize];
    let carrying = state.figure_carries[fig as usize].is_some();
    let in_spuk_room = state.node_states[pos as usize].has_spuk;

    ADJACENCY.edges_of(pos).iter().copied().filter(move |&e| {
        if !edge_passable(state, e) {
            return false;
        }
        // Spuk-trap: a figure carrying a jewel cannot leave a Spuk room.
        if carrying && in_spuk_room {
            return false;
        }
        // Occupied hallway cells may be entered (pass-through) but not as a final stop.
        // We still allow the edge; StopMoving is suppressed separately when on such a cell.
        true
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
            let fig = state.active_figure as usize;
            let pos = state.figure_pos[fig];
            // May only stop if not currently on an occupied hallway cell (pass-through rule).
            let on_occupied_hallway =
                matches!(crate::board::NODES[pos as usize].kind, NodeKind::Hallway)
                    && state.node_states[pos as usize].figure_count() > 1;
            if !on_occupied_hallway {
                buf.push(Action::StopMoving);
                if state.figure_carries[fig].is_some() && pos == ENTRANCE {
                    buf.push(Action::DepositJewel);
                }
            }
        }
        TurnPhase::PickupJewel => {
            // Deposit first if at entrance with a jewel (can happen if movement ended there).
            let fig = state.active_figure as usize;
            if state.figure_carries[fig].is_some() && state.figure_pos[fig] == ENTRANCE {
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
/// Returns `None` if unreachable. Respects closed edges, pass-through, and the Spuk-trap.
pub fn bfs_distance<F>(state: &GameState, start: NodeId, goal: F) -> Option<u32>
where
    F: Fn(NodeId) -> bool,
{
    if goal(start) {
        return Some(0);
    }
    let carrying = state.figure_carries[state.active_figure as usize].is_some();
    let mut visited = [false; NODE_COUNT];
    visited[start as usize] = true;
    let mut queue: std::collections::VecDeque<(NodeId, u32)> = std::collections::VecDeque::new();
    queue.push_back((start, 0));
    while let Some((node, dist)) = queue.pop_front() {
        // Spuk-trap: cannot leave a Spuk room while carrying a jewel.
        if carrying && state.node_states[node as usize].has_spuk {
            continue;
        }
        for &edge in ADJACENCY.edges_of(node) {
            if !edge_passable(state, edge) {
                continue;
            }
            let next = ADJACENCY.other_end(edge, node);
            if visited[next as usize] {
                continue;
            }
            visited[next as usize] = true;
            // An occupied hallway cell can be traversed but not used as a goal.
            let occupied_hallway =
                matches!(crate::board::NODES[next as usize].kind, NodeKind::Hallway)
                    && state.node_states[next as usize].figure_count() > 0;
            if !occupied_hallway && goal(next) {
                return Some(dist + 1);
            }
            queue.push_back((next, dist + 1));
        }
    }
    None
}
