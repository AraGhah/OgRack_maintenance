use super::super::{Game, GameState, HostJoinFocus, Input};

/// Gère les interactions de la scène héberger/rejoindre
pub fn update(game: &mut Game, input: &Input) {
    if input.tab {
        game.host_join_focus = match game.host_join_focus {
            HostJoinFocus::Identifier => HostJoinFocus::Host,
            HostJoinFocus::Host => HostJoinFocus::Join,
            HostJoinFocus::Join => HostJoinFocus::Identifier,
        };
    }

    let mut edited = false;
    if game.host_join_focus == HostJoinFocus::Identifier {
        if let Some(character) = input.character
            && crate::message::is_valid_player_id_character(character)
            && game.my_id.len() < 32
        {
            game.my_id.push(character);
            edited = true;
        }

        if input.backspace {
            edited |= game.my_id.pop().is_some();
        }
    }

    if edited {
        game.connection_error = None;
    }

    if input.enter && game.host_join_focus != HostJoinFocus::Identifier {
        if !crate::message::is_valid_player_id(&game.my_id) {
            game.connection_error =
                Some("Choisissez un identifiant valide (1 à 32 caractères).".to_string());
            return;
        }

        game.is_host = game.host_join_focus == HostJoinFocus::Host;
        if game.is_host {
            game.host_ip = crate::client::get_local_ip().expect("Failed to retrieve local IP");
        }
        
        game.connection_error = None;
        game.set_state(GameState::Lobby);
    }
}
