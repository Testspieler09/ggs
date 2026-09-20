use ggs_core::action::Action;
use ggs_core::board::EDGE_COUNT;
use ggs_core::observation::{observation_vector, PlayerView, OBS_DIM};
use ggs_core::state::JEWEL_COUNT;
use ggs_core::strategy_trait::Strategy;
use ggs_core::variant::Variant;

// Action vocabulary layout (61 total):
//   0          RollDie
//   1          DrawGhostCard
//   2..=46     MoveAlongEdge { edge: 0..=44 }
//   47         StopMoving
//   48..=55    PickupJewel { jewel: 0..=7 }
//   56         SkipPickup
//   57         DepositJewel
//   58         Fight
//   59         SkipCombat
//   60         EndTurn
pub const ACTION_DIM: usize = 61;

const MOVE_OFFSET: usize = 2;
const STOP_IDX: usize = MOVE_OFFSET + EDGE_COUNT;
const PICKUP_OFFSET: usize = STOP_IDX + 1;
const SKIP_PICKUP_IDX: usize = PICKUP_OFFSET + JEWEL_COUNT;
const DEPOSIT_IDX: usize = SKIP_PICKUP_IDX + 1;
const FIGHT_IDX: usize = DEPOSIT_IDX + 1;
const SKIP_COMBAT_IDX: usize = FIGHT_IDX + 1;
const END_TURN_IDX: usize = SKIP_COMBAT_IDX + 1;

fn action_to_idx(action: Action) -> usize {
    match action {
        Action::RollDie => 0,
        Action::DrawGhostCard => 1,
        Action::MoveAlongEdge { edge } => MOVE_OFFSET + edge as usize,
        Action::StopMoving => STOP_IDX,
        Action::PickupJewel { jewel } => PICKUP_OFFSET + jewel as usize,
        Action::SkipPickup => SKIP_PICKUP_IDX,
        Action::DepositJewel => DEPOSIT_IDX,
        Action::Fight => FIGHT_IDX,
        Action::SkipCombat => SKIP_COMBAT_IDX,
        Action::EndTurn => END_TURN_IDX,
    }
}

fn idx_to_action(idx: usize) -> Action {
    match idx {
        0 => Action::RollDie,
        1 => Action::DrawGhostCard,
        STOP_IDX => Action::StopMoving,
        SKIP_PICKUP_IDX => Action::SkipPickup,
        DEPOSIT_IDX => Action::DepositJewel,
        FIGHT_IDX => Action::Fight,
        SKIP_COMBAT_IDX => Action::SkipCombat,
        END_TURN_IDX => Action::EndTurn,
        i if (MOVE_OFFSET..STOP_IDX).contains(&i) => Action::MoveAlongEdge {
            edge: (i - MOVE_OFFSET) as u8,
        },
        i if (PICKUP_OFFSET..SKIP_PICKUP_IDX).contains(&i) => Action::PickupJewel {
            jewel: (i - PICKUP_OFFSET) as u8,
        },
        _ => panic!("invalid action index {idx}"),
    }
}

// ---------------------------------------------------------------------------
// MLP: 259 -> 256 -> 128 -> 61
// Weights stored flat: [W0 | b0 | W1 | b1 | W2 | b2]
// Each layer: W is (out x in) row-major, b is (out).
// ---------------------------------------------------------------------------

const H1: usize = 256;
const H2: usize = 128;

pub const WEIGHT_COUNT: usize = H1 * OBS_DIM + H1          // layer 0
    + H2 * H1 + H2             // layer 1
    + ACTION_DIM * H2 + ACTION_DIM; // layer 2

/// Reinforcement learning strategy: a small MLP policy network.
///
/// Forward pass: observation_vector -> Linear(259,256) -> ReLU
///                                  -> Linear(256,128) -> ReLU
///                                  -> Linear(128,61)  -> mask illegals -> softmax -> argmax
///
/// Weights are stored flat (little-endian f32) and can be loaded from a file
/// produced by the training loop in `train/src/main.rs`.
#[derive(Clone)]
pub struct RlStrategy {
    weights: Vec<f32>,
}

impl RlStrategy {
    /// Create with random (Xavier-initialised) weights - useful for initial self-play.
    pub fn new(seed: u64) -> Self {
        use rand::rngs::SmallRng;
        use rand::{Rng, SeedableRng};

        let mut rng = SmallRng::seed_from_u64(seed);
        let mut weights = vec![0.0f32; WEIGHT_COUNT];

        let mut offset = 0;
        // Xavier uniform init per layer: limit = sqrt(6 / (fan_in + fan_out))
        for (fan_in, fan_out) in [(OBS_DIM, H1), (H1, H2), (H2, ACTION_DIM)] {
            let limit = (6.0 / (fan_in + fan_out) as f32).sqrt();
            let w_count = fan_out * fan_in;
            for w in &mut weights[offset..offset + w_count] {
                *w = rng.random_range(-limit..=limit);
            }
            offset += w_count;
            // biases stay zero
            offset += fan_out;
        }

        Self { weights }
    }

    /// Load weights from a flat little-endian f32 binary file.
    pub fn load(path: &std::path::Path) -> std::io::Result<Self> {
        let bytes = std::fs::read(path)?;
        if bytes.len() != WEIGHT_COUNT * 4 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("expected {} bytes, got {}", WEIGHT_COUNT * 4, bytes.len()),
            ));
        }
        let weights = bytes
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect();
        Ok(Self { weights })
    }

    /// Save weights to a flat little-endian f32 binary file.
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let bytes: Vec<u8> = self.weights.iter().flat_map(|w| w.to_le_bytes()).collect();
        std::fs::write(path, bytes)
    }

    /// Expose weights mutably for the training loop (gradient updates).
    pub fn weights_mut(&mut self) -> &mut [f32] {
        &mut self.weights
    }

    pub fn weights(&self) -> &[f32] {
        &self.weights
    }

    /// Run the forward pass. Returns raw logits over the 61-action vocabulary.
    fn forward(&self, obs: &[f32]) -> [f32; ACTION_DIM] {
        let mut offset = 0;

        // Layer 0: (H1 x OBS_DIM) + bias H1
        let mut h1 = [0.0f32; H1];
        linear_relu(obs, OBS_DIM, &mut h1, H1, &self.weights, &mut offset);

        // Layer 1: (H2 x H1) + bias H2
        let mut h2 = [0.0f32; H2];
        linear_relu(&h1, H1, &mut h2, H2, &self.weights, &mut offset);

        // Layer 2: (ACTION_DIM x H2) + bias ACTION_DIM - no activation
        let mut logits = [0.0f32; ACTION_DIM];
        linear(&h2, H2, &mut logits, ACTION_DIM, &self.weights, &mut offset);

        logits
    }

    /// Choose the legal action with the highest post-softmax probability.
    fn masked_argmax(&self, logits: &[f32; ACTION_DIM], legal: &[Action]) -> Action {
        // Shift by max for numerical stability before softmax, then mask illegals.
        let legal_indices: Vec<usize> = legal.iter().map(|&a| action_to_idx(a)).collect();

        let max_logit = legal_indices
            .iter()
            .map(|&i| logits[i])
            .fold(f32::NEG_INFINITY, f32::max);

        let best_idx = legal_indices
            .iter()
            .copied()
            .max_by(|&a, &b| {
                let ea = (logits[a] - max_logit).exp();
                let eb = (logits[b] - max_logit).exp();
                ea.partial_cmp(&eb).unwrap()
            })
            .unwrap();

        idx_to_action(best_idx)
    }
}

// ---------------------------------------------------------------------------
// Low-level linear algebra helpers
// ---------------------------------------------------------------------------

/// y = ReLU(W x + b). W is (out x in) row-major. Advances `offset` past W and b.
#[inline]
fn linear_relu(
    x: &[f32],
    in_dim: usize,
    y: &mut [f32],
    out_dim: usize,
    weights: &[f32],
    offset: &mut usize,
) {
    let w = &weights[*offset..*offset + out_dim * in_dim];
    *offset += out_dim * in_dim;
    let b = &weights[*offset..*offset + out_dim];
    *offset += out_dim;

    for (row, (yi, &bi)) in y.iter_mut().zip(b.iter()).enumerate() {
        let dot: f32 = (0..in_dim).map(|col| w[row * in_dim + col] * x[col]).sum();
        *yi = (dot + bi).max(0.0); // ReLU
    }
}

/// y = W x + b. No activation. Advances `offset` past W and b.
#[inline]
fn linear(
    x: &[f32],
    in_dim: usize,
    y: &mut [f32],
    out_dim: usize,
    weights: &[f32],
    offset: &mut usize,
) {
    let w = &weights[*offset..*offset + out_dim * in_dim];
    *offset += out_dim * in_dim;
    let b = &weights[*offset..*offset + out_dim];
    *offset += out_dim;

    for (row, (yi, &bi)) in y.iter_mut().zip(b.iter()).enumerate() {
        let dot: f32 = (0..in_dim).map(|col| w[row * in_dim + col] * x[col]).sum();
        *yi = dot + bi;
    }
}

impl Strategy for RlStrategy {
    fn choose_action(&mut self, view: &PlayerView) -> Action {
        let obs = observation_vector(view);
        let logits = self.forward(&obs);
        self.masked_argmax(&logits, &view.legal_actions)
    }

    fn on_game_start(&mut self, _figure_id: u8, _variant: Variant) {}

    fn on_game_end(&mut self, _won: bool) {}
}
