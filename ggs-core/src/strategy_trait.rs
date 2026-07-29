use crate::action::Action;
use crate::observation::PlayerView;
use crate::variant::Variant;

/// Every AI or human controller implements this trait.
/// `choose_action` is called exactly once per decision point.
/// The returned action must appear in `view.legal_actions`;
/// returning an illegal action causes a panic in debug mode.
pub trait Strategy: Send + Sync {
    fn choose_action(&self, view: &PlayerView) -> Action;

    /// Called once before the first turn of a new game.
    fn on_game_start(&mut self, figure_id: u8, variant: Variant) {
        let _ = (figure_id, variant);
    }

    /// Called once after the game ends.
    fn on_game_end(&mut self, won: bool) {
        let _ = won;
    }
}
