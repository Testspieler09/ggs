use ggs_core::action::Action;
use ggs_core::observation::PlayerView;
use ggs_core::strategy_trait::Strategy;
use ggs_core::variant::Variant;

/// Reinforcement learning strategy stub.
///
/// Intended approach: policy-gradient (PPO) with `observation_vector` as input
/// to a small MLP. The policy head outputs logits over the action vocabulary;
/// illegal actions are masked before softmax.
///
/// The training loop lives in `train/src/main.rs`.
/// This struct holds flat MLP weights and performs forward-pass inference.
///
/// TODO: implement MLP forward pass, action masking, softmax argmax.
pub struct RlStrategy {
    /// Flattened MLP weights (layer by layer, little-endian f32).
    pub weights: Vec<f32>,
    pub obs_dim: usize,
    pub action_dim: usize,
}

impl RlStrategy {
    pub fn new(obs_dim: usize, action_dim: usize) -> Self {
        Self {
            weights: Vec::new(),
            obs_dim,
            action_dim,
        }
    }

    /// Load weights from a flat binary file (little-endian f32).
    pub fn load_weights(&mut self, path: &std::path::Path) -> std::io::Result<()> {
        let bytes = std::fs::read(path)?;
        self.weights = bytes
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect();
        Ok(())
    }
}

impl Strategy for RlStrategy {
    fn choose_action(&self, view: &PlayerView) -> Action {
        // Stub: fall back to first legal action until MLP is implemented.
        view.legal_actions
            .first()
            .copied()
            .expect("no legal actions")
    }

    fn on_game_start(&mut self, _figure_id: u8, _variant: Variant) {}

    fn on_game_end(&mut self, _won: bool) {
        // In a real implementation: store episode result for policy update.
    }
}
