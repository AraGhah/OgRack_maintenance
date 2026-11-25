use super::super::{Game, GameState, Input};

/// Gère les interactions de la scène titre
pub fn update(game: &mut Game, input: &Input) {
    if input.enter {
        game.set_state(GameState::HostJoin);
    }
}
