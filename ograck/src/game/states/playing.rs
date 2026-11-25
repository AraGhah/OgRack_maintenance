use super::super::{Game, Input};
use crate::board::Board;
use crate::game::MovementSpeed;
use crate::message::Message;
use macroquad::time::get_frame_time;

/// Gère les interactions de la scène d'action
pub fn update(game: &mut Game, input: &Input) {
    if let Some(client) = &game.client {
        if !game.obstacles_requested {
            let request = Message::C2SObstacles {
                id: game.my_id.clone(),
            };
            
            match client.send(&request) {
                Ok(()) => game.obstacles_requested = true,
                Err(error) => eprintln!("Échec de récupération des obstacles: {error}"),
            }
        }

        if game.last_sent_zombie_state != Some(game.is_zombie) {
            let message = Message::C2SZombieState {
                id: game.my_id.clone(),
                is_zombie: game.is_zombie,
            };

            match client.send(&message) {
                Ok(()) => game.last_sent_zombie_state = Some(game.is_zombie),
                Err(error) => eprintln!("Échec d'envoi de l'état zombie au serveur: {error}"),
            }
        }

        loop {
            match client.receive() {
                Ok(Some(Message::S2CObstacles { obstacles })) => {
                    if game.board.is_none() {
                        game.board = Some(Board::from(obstacles));
                        if let Some((x, y)) = game.board.as_ref().and_then(Board::random_empty_tile)
                        {
                            game.position = [x as f32, y as f32];
                        }
                    }
                }
                Ok(Some(Message::S2CPosition { id, x, y })) => {
                    if id != game.my_id
                        && game
                            .board
                            .as_ref()
                            .is_some_and(|board| board.is_empty_tile(x, y))
                    {
                        game.other_players.insert(id, (x, y));
                    }
                }
                Ok(Some(Message::S2CZombieState { id, is_zombie })) => {
                    if id != game.my_id {
                        game.other_player_zombie_states.insert(id, is_zombie);
                    }
                }
                Ok(Some(Message::S2CZombify)) => {
                    game.movement_speed = MovementSpeed::Slow;
                    game.is_zombie = true;
                }
                Ok(Some(_)) => {}
                Ok(None) => break,
                Err(error) => {
                    eprintln!("Échec de réception des messages en provenance du serveur: {error}");
                    break;
                }
            }
        }

        if let Some(board) = &game.board {
            if game.movement_target.is_none() {
                let direction = (
                    axis_direction(input.right, input.left),
                    axis_direction(input.down, input.up),
                );

                game.movement_target = adjacent_empty_tile(board, game.position, direction);
            }

            if let Some(target) = game.movement_target {
                let distance = game.movement_speed.tiles_per_second() * get_frame_time();
                
                if advance_toward_tile(&mut game.position, target, distance) {
                    game.position = [target.0 as f32, target.1 as f32];
                    game.movement_target = None;
                }
            }

            let position = (
                game.position[0].round() as usize,
                game.position[1].round() as usize,
            );

            if game.last_sent_position != Some(position) {
                let message = Message::C2SPosition {
                    id: game.my_id.clone(),
                    x: position.0,
                    y: position.1,
                };

                match client.send(&message) {
                    Ok(()) => game.last_sent_position = Some(position),
                    Err(error) => eprintln!("Échec d'envoi de la position au serveur: {error}"),
                }
            }
        }
    }
}

/// Renvoie la direction d'un axe sous forme de nombre
fn axis_direction(positive: bool, negative: bool) -> isize {
    match (positive, negative) {
        (true, false) => 1,
        (false, true) => -1,
        _ => 0,
    }
}

/// Cherche la tuile voisine vers laquelle le personnage peut se déplacer.
/// 
/// Tient compte de la direction du personnage. 
/// Renvoie l'emplacement de la tuile voisine (Some) si elle est libre ou None
/// si le déplacement est impossible. 
fn adjacent_empty_tile(
    board: &Board,
    position: [f32; 2],
    direction: (isize, isize),
) -> Option<(usize, usize)> {
    if direction == (0, 0) {
        return None;
    }

    let current_x = position[0].round() as usize;
    let current_y = position[1].round() as usize;
    let next_x = current_x.checked_add_signed(direction.0)?;
    let next_y = current_y.checked_add_signed(direction.1)?;

    if board.is_obstacle(next_x, next_y) != Some(false) {
        return None;
    }

    if direction.0 != 0
        && direction.1 != 0
        && (board.is_obstacle(next_x, current_y) != Some(false)
            || board.is_obstacle(current_x, next_y) != Some(false))
    {
        return None;
    }

    Some((next_x, next_y))
}

/// Fait avancer progressivement le joueur vers une tuile cible
/// 
/// Si le personnage atteint la tuile cible lors du déplacement, la fonction le place 
/// sur la tuile et retourne true.
/// Les déplacements sont en nombres décimaux, les positions sont entières. 
fn advance_toward_tile(position: &mut [f32; 2], target: (usize, usize), distance: f32) -> bool {
    let delta_x = target.0 as f32 - position[0];
    let delta_y = target.1 as f32 - position[1];
    let remaining = (delta_x * delta_x + delta_y * delta_y).sqrt();

    if remaining <= distance {
        position[0] = target.0 as f32;
        position[1] = target.1 as f32;

        true
    } else {
        position[0] += delta_x / remaining * distance;
        position[1] += delta_y / remaining * distance;

        false
    }
}
