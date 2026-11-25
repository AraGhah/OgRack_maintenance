/// Monde dans lequel les joueurs évoluent.
///
/// Le monde est autoritaire en ce sens qu'il détient la vérité sur l'état général de
/// la simulation.
///
/// Il est responsable de la gestion de tout ce qui se passe dans la simulation et
/// diffuse les informations aux clients connectés. La simulation communique avec les
/// joueurs via le serveur qui agit comme intermédiaire entre le monde et les clients.
///
/// La simulation s'exécute dans son propre fil d'exécution
/// 
use std::collections::{HashMap, HashSet};
use std::io;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::board::Board;
use crate::message::Message;
use crate::server::Server;

const WORLD_TICK: Duration = Duration::from_millis(20);
const INITIAL_ZOMBIE_DELAY: Duration = Duration::from_secs(10);
const ZOMBIFY_RETRY_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Default)]
struct PlayerState {
    position: Option<(usize, usize)>,
    is_zombie: bool,
    is_ready: bool,
    is_host: bool,
}

pub struct World {
    shutdown: mpsc::Sender<()>,
    worker: Option<JoinHandle<io::Result<()>>>,
}

impl World {
    pub fn start(server: Arc<Server>) -> Self {
        let (shutdown_tx, shutdown_rx) = mpsc::channel();
        let worker = thread::spawn(move || run(server, shutdown_rx));

        Self {
            shutdown: shutdown_tx,
            worker: Some(worker),
        }
    }
}

impl Drop for World {
    fn drop(&mut self) {
        let _ = self.shutdown.send(());

        if let Some(worker) = self.worker.take() {
            match worker.join() {
                Ok(Ok(())) => {}
                Ok(Err(error)) => eprintln!("Arrêt de la simulation avec erreur: {error}"),
                Err(_) => eprintln!("Panique du fil d'exécution de la simulation"),
            }
        }
    }
}

/// Fil d'exécution de la simulation
fn run(server: Arc<Server>, shutdown: Receiver<()>) -> io::Result<()> {
    let mut players = HashMap::<String, PlayerState>::new();
    let mut pending_zombifications = HashMap::<String, Instant>::new();
    let mut simulation_started = None;
    let mut initial_zombie_selected = false;
    let mut game_started = false;

    let board = Board::new();

    loop {
        match shutdown.try_recv() {
            Ok(()) | Err(TryRecvError::Disconnected) => return Ok(()),
            Err(TryRecvError::Empty) => {}
        }

        while let Some(message) = server.receive()? {
            match message {
                Message::C2SPosition { id, x, y } => {
                    if board.is_empty_tile(x, y) {
                        let player = players.entry(id.clone()).or_default();
                        let previous_position = player.position;
                        
                        player.position = Some((x, y));
                        
                        if position_needs_broadcast(previous_position, (x, y)) {
                            let update = Message::S2CPosition {
                                id: id.clone(),
                                x,
                                y,
                            };

                            server.send_to_all_except(&id, &update)?;
                        }
                    }
                }
                Message::C2SZombieState { id, is_zombie } => {
                    let player = players.entry(id.clone()).or_default();
                    let state_changed = player.is_zombie != is_zombie;
                    
                    player.is_zombie = is_zombie;
                    
                    if is_zombie {
                        pending_zombifications.remove(&id);
                    }

                    if state_changed {
                        let update = Message::S2CZombieState {
                            id: id.clone(),
                            is_zombie,
                        };
                        server.send_to_all_except(&id, &update)?;
                    }
                }
                Message::C2SLobbyState {
                    id,
                    is_ready,
                    is_host,
                } => {
                    let is_new_player = !players.contains_key(&id);
                    
                    let previous_players = if is_new_player {
                        players
                            .iter()
                            .filter(|(player_id, _)| *player_id != &id)
                            .map(|(player_id, player)| {
                                (player_id.clone(), player.is_ready, player.is_host)
                            })
                            .collect()
                    } else {
                        Vec::new()
                    };

                    let player = players.entry(id.clone()).or_default();
                    let state_changed = player.is_ready != is_ready || player.is_host != is_host;
                    
                    player.is_ready = is_ready;
                    player.is_host = is_host;

                    for (player_id, ready, host) in previous_players {
                        server.send_to_one(
                            &id,
                            &Message::S2CLobbyPlayer {
                                id: player_id,
                                is_ready: ready,
                                is_host: host,
                            },
                        )?;
                    }

                    let update = Message::S2CLobbyPlayer {
                        id: id.clone(),
                        is_ready,
                        is_host,
                    };

                    if is_new_player || state_changed {
                        for player_id in players.keys() {
                            server.send_to_one(player_id, &update)?;
                        }
                    } else {
                        server.send_to_one(&id, &update)?;
                    }
                }
                Message::C2SStartGame { id } => {
                    if can_start_game(&id, &players) {
                        if !game_started {
                            simulation_started = Some(Instant::now());
                        }

                        game_started = true;
                    }

                    if game_started {
                        for player_id in players.keys() {
                            server.send_to_one(player_id, &Message::S2CStartGame)?;
                        }
                    }
                }
                Message::C2SObstacles { id } => {
                    let obstacles = board.serialize();

                    server.send_to_one(&id, &Message::S2CObstacles { obstacles })?;
                    
                    for (player_id, player) in &players {
                        if player_id != &id
                            && let Some((x, y)) = player.position
                        {
                            server.send_to_one(
                                &id,
                                &Message::S2CPosition {
                                    id: player_id.clone(),
                                    x,
                                    y,
                                },
                            )?;
                        }

                        server.send_to_one(
                            &id,
                            &Message::S2CZombieState {
                                id: player_id.clone(),
                                is_zombie: player.is_zombie,
                            },
                        )?;
                    }
                }
                _ => {}
            }
        }

        let mut pending_ids: HashSet<String> = pending_zombifications.keys().cloned().collect();
        
        if !initial_zombie_selected
            && simulation_started.is_some_and(|started| started.elapsed() >= INITIAL_ZOMBIE_DELAY)
            && let Some(id) = choose_initial_zombie(&players, &pending_ids)
        {
            server.send_to_one(&id, &Message::S2CZombify)?;

            pending_zombifications.insert(id.clone(), Instant::now());
            pending_ids.insert(id);

            initial_zombie_selected = true;
        }

        for id in find_zombification_candidates(&players, &pending_ids) {
            server.send_to_one(&id, &Message::S2CZombify)?;
            pending_zombifications.insert(id, Instant::now());
        }

        let now = Instant::now();
        for (id, last_sent) in &mut pending_zombifications {
            if now.duration_since(*last_sent) >= ZOMBIFY_RETRY_INTERVAL {
                server.send_to_one(id, &Message::S2CZombify)?;

                *last_sent = now;
            }
        }

        thread::sleep(WORLD_TICK);
    }
}

/// Détermine si la partie peut être lancée
/// 
/// Une partie peut être lancée par le joueur qui héberge la partie si tous les
/// joueurs sont prêts.
fn can_start_game(requester_id: &str, players: &HashMap<String, PlayerState>) -> bool {
    !players.is_empty()
        && players
            .get(requester_id)
            .is_some_and(|player| player.is_host)
        && players.values().all(|player| player.is_ready)
}

/// Vérifie s'il faut diffuser un changement de position
fn position_needs_broadcast(
    previous_position: Option<(usize, usize)>,
    new_position: (usize, usize),
) -> bool {
    previous_position != Some(new_position)
}

/// Sélectionne le personnage qui deviendra le permier zombie
fn choose_initial_zombie(
    players: &HashMap<String, PlayerState>,
    pending: &HashSet<String>,
) -> Option<String> {
    let eligible: Vec<&String> = players
        .iter()
        .filter(|(id, player)| !player.is_zombie && !pending.contains(*id))
        .map(|(id, _)| id)
        .collect();

    if eligible.is_empty() {
        return None;
    }

    let index = rand::random_range(0..eligible.len());

    Some(eligible[index].clone())
}

/// Trouve les personnages qui partagent la même position qu'un zombie
fn find_zombification_candidates(
    players: &HashMap<String, PlayerState>,
    pending: &HashSet<String>,
) -> Vec<String> {
    players
        .iter()
        .filter_map(|(id, player)| {
            let position = player.position?;
            
            if player.is_zombie || pending.contains(id) {
                return None;
            }

            players
                .values()
                .any(|other| other.is_zombie && other.position == Some(position))
                .then(|| id.clone())
        })
        .collect()
}
