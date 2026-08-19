use ggs_core::action::Action;
use ggs_core::board::{room_node_id, NodeId, NodeKind, ADJACENCY, NODES};
use ggs_core::observation::PlayerView;

use ggs_core::strategy_trait::Strategy;

/// Heuristic greedy strategy.
///
/// Priority order per turn:
/// 1. Deposit jewel at entrance if carrying one and already at entrance.
/// 2. Pick up a jewel if in a jewel room and hands are free (correct number if variant active).
/// 3. Fight: skip combat only if no ghosts/Spuk or alone with Spuk.
/// 4. Move: prefer direction that minimises BFS distance to nearest jewel goal.
///    Goal = nearest uncollected jewel room if empty-handed,
///    or entrance if carrying a jewel.
/// 5. Fall back to first legal action.
#[derive(Clone)]
pub struct GreedyStrategy;

impl GreedyStrategy {
    /// BFS distance from `start` to the nearest node satisfying `goal`, using
    /// only the passable edges encoded in the player view.
    pub fn bfs_to_goal(
        view: &PlayerView,
        start: NodeId,
        is_goal: impl Fn(NodeId) -> bool,
    ) -> Option<u32> {
        use std::collections::VecDeque;
        if is_goal(start) {
            return Some(0);
        }
        const N: usize = NODES.len();
        let mut visited = vec![false; N];
        visited[start as usize] = true;
        let mut queue: VecDeque<(NodeId, u32)> = VecDeque::new();
        queue.push_back((start, 0));
        while let Some((node, dist)) = queue.pop_front() {
            for &edge in ADJACENCY.edges_of(node) {
                if view.edge_closed[edge as usize] {
                    continue;
                }
                let next = ADJACENCY.other_end(edge, node);
                if visited[next as usize] {
                    continue;
                }
                // Hallway occupancy (ignore active figure's own position).
                if matches!(NODES[next as usize].kind, NodeKind::Hallway) {
                    let other_figures =
                        view.node_states[next as usize].figures & !(1 << view.figure_id);
                    let no_figure_present = other_figures != 0;
                    if no_figure_present {
                        continue;
                    }
                }
                visited[next as usize] = true;
                if is_goal(next) {
                    return Some(dist + 1);
                }
                queue.push_back((next, dist + 1));
            }
        }
        None
    }
}

impl Strategy for GreedyStrategy {
    fn choose_action(&self, view: &PlayerView) -> Action {
        use ggs_core::action::Action::*;
        use ggs_core::board::ENTRANCE;

        let legal = &view.legal_actions;

        // --- Single-choice phases: just return the only option ---
        if legal.len() == 1 {
            return legal[0];
        }

        let fig = view.figure_id as usize;
        let pos = view.figure_positions[fig];
        let carrying = view.figure_carries[fig];

        // 1. Deposit jewel if possible.
        if legal.contains(&DepositJewel) {
            return DepositJewel;
        }

        // 2. Pick up a jewel if available.
        // FIX: we need to check for a softlock on hard mode here
        // Meaning if all figures have Jewels in their back but cannot deposit them
        // as the order doesn't match.
        for &a in legal {
            if matches!(a, PickupJewel { .. }) {
                return a;
            }
        }

        // 3. Fight if we can (and there's an explicit Fight action available).
        if legal.contains(&Fight) {
            return Fight;
        }

        // 4. Move: pick the edge that reduces distance to goal most.
        let goal_node: Option<NodeId> = if carrying.is_some() {
            // Heading to entrance.
            Some(ENTRANCE)
        } else {
            // Find nearest room with an uncollected jewel.
            let mut best_dist = u32::MAX;
            let mut best_node = None;
            for room in ggs_core::board::RoomLabel::STARTS_WITH_JEWEL {
                let rn = room_node_id(room);
                let ns = &view.node_states[rn as usize];
                if ns.jewel.is_none() {
                    continue; // already collected
                }
                if let Some(d) = Self::bfs_to_goal(view, pos, |n| n == rn) {
                    if d < best_dist {
                        best_dist = d;
                        best_node = Some(rn);
                    }
                }
            }
            best_node
        };

        if let Some(target) = goal_node {
            // Among MoveAlongEdge options, pick the one that gets us closer to target.
            let mut best_action = None;
            let mut best_dist = u32::MAX;
            for &a in legal {
                if let MoveAlongEdge { edge } = a {
                    let dest = ADJACENCY.other_end(edge, pos);
                    if let Some(d) = Self::bfs_to_goal(view, dest, |n| n == target) {
                        if d < best_dist {
                            best_dist = d;
                            best_action = Some(a);
                        }
                    }
                }
            }
            if let Some(a) = best_action {
                return a;
            }
        }

        // TODO: check if this holds true otherwise revert to fallback[0]
        unreachable!("At this point an action should be picked!");
    }
}
