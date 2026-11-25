use std::sync::Arc;

use super::super::{Game, GameState, Input};
use crate::client::Client;
use crate::message::Message;
use crate::server::{SERVER_PORT, Server};
use crate::world::World;
use macroquad::time::get_time;

const LOBBY_STATE_RETRY_SECONDS: f64 = 0.5;

/// Gère les interactions de la salle d'attente
pub fn update(game: &mut Game, input: &Input) {
    if game.client.is_none() {
        if let Some(key) = input.character
            && (key.is_ascii_digit() || key == '.') {
            game.input_ip.push(key);
        }

        if input.backspace {
            game.input_ip.pop();
        }

        if !input.enter {
            return;
        }

        if game.is_host {
            match Server::start() {
                Ok(server) => {
                    match Client::connect(&format!("127.0.0.1:{SERVER_PORT}"), Some(&game.my_id)) {
                        Ok(client) => {
                            let server = Arc::new(server);
                            game.world = Some(World::start(Arc::clone(&server)));
                            game.my_id = client.client_id.clone();
                            game.server = Some(server);
                            game.client = Some(client);
                        }
                        Err(error) => {
                            eprintln!("Échec au démarrage du client: {error}");
                            game.connection_error = Some(error.to_string());
                            game.set_state(GameState::HostJoin);
                        }
                    }
                }
                Err(error) => {
                    eprintln!("Échec de démarrage du serveur: {error}");
                    game.connection_error = Some(error.to_string());
                    game.set_state(GameState::HostJoin);
                }
            }
        } else {
            match Client::connect(&game.input_ip, Some(&game.my_id)) {
                Ok(client) => {
                    game.my_id = client.client_id.clone();
                    game.client = Some(client);
                }
                Err(error) => {
                    eprintln!("Ne peut rejoindre le serveur: {error}");
                    game.connection_error = Some(if error == "Identifiant indisponible" {
                        "Cet identifiant est déjà pris. Choisissez-en un autre.".to_string()
                    } else {
                        error
                    });
                    game.set_state(GameState::HostJoin);
                }
            }
        }
        return;
    }

    let client = game.client.as_ref().expect("client was checked above");
    loop {
        match client.receive() {
            Ok(Some(Message::S2CLobbyPlayer {
                id,
                is_ready,
                is_host,
            })) => {
                if id == game.my_id && is_ready == game.is_ready && is_host == game.is_host {
                    game.lobby_state_sent = true;
                }
                game.lobby_players
                    .insert(id, super::super::LobbyPlayer { is_ready, is_host });
            }
            Ok(Some(Message::S2CStartGame)) => {
                game.set_state(GameState::Playing);
                return;
            }
            Ok(Some(_)) => {}
            Ok(None) => break,
            Err(error) => {
                eprintln!("Échec de mise à jour de la salle d'attente: {error}");
                break;
            }
        }
    }

    if input.ready {
        game.is_ready = !game.is_ready;
        game.lobby_state_sent = false;
    }

    let now = get_time();
    if !game.lobby_state_sent && now - game.lobby_state_last_sent >= LOBBY_STATE_RETRY_SECONDS {
        let message = Message::C2SLobbyState {
            id: game.my_id.clone(),
            is_ready: game.is_ready,
            is_host: game.is_host,
        };
        match client.send(&message) {
            Ok(()) => game.lobby_state_last_sent = now,
            Err(error) => eprintln!("Échec d'enregistrement de l'état d'attente: {error}"),
        }
    }

    let all_ready =
        !game.lobby_players.is_empty() && game.lobby_players.values().all(|player| player.is_ready);
    if game.is_host && input.start && all_ready {
        match client.send(&Message::C2SStartGame {
            id: game.my_id.clone(),
        }) {
            Ok(()) => {}
            Err(error) => eprintln!("Échec au démarrage de la partie: {error}"),
        }
    }
}
