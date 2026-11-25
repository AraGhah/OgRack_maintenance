use std::collections::HashMap;

use std::sync::Arc;

use crate::board::Board;
use crate::client::Client;
use crate::server::Server;
use crate::world::World;

pub mod states;

pub const WINDOW_WIDTH: usize = 1104;
pub const WINDOW_HEIGHT: usize = 736;
pub const TILE_SIZE: usize = 16;

/// Les différents états du jeu
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    Intro,
    HostJoin,
    Lobby,
    Playing,
}

/// Champ qui a le focus dans la scène héberger/rejoindre
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HostJoinFocus {
    Identifier,
    Host,
    Join,
}

/// Vitesses de déplacement du personnage
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MovementSpeed {
    Slow,
    Fast,
}

impl MovementSpeed {
    pub fn tiles_per_second(self) -> f32 {
        match self {
            Self::Slow => 6.0,
            Self::Fast => 10.0,
        }
    }
}

/// Toutes les données du jeu
pub struct Game {
    state: GameState,
    intro_started_at: f64,

    pub my_id: String,
    pub connection_error: Option<String>,
    pub host_join_focus: HostJoinFocus,
    pub is_host: bool,
    pub is_ready: bool,
    pub lobby_players: HashMap<String, LobbyPlayer>,
    pub lobby_state_sent: bool,
    pub lobby_state_last_sent: f64,
    pub input_ip: String,
    pub host_ip: String,

    pub is_zombie: bool,
    pub position: [f32; 2],
    pub movement_speed: MovementSpeed,
    pub movement_target: Option<(usize, usize)>,
    pub other_players: HashMap<String, (usize, usize)>,
    pub other_player_zombie_states: HashMap<String, bool>,
    pub last_sent_position: Option<(usize, usize)>,
    pub last_sent_zombie_state: Option<bool>,

    pub client: Option<Client>,
    pub world: Option<World>,
    pub server: Option<Arc<Server>>,
    pub board: Option<Board>,
    pub obstacles_requested: bool,
}

/// État d'un joueur dans la salle d'attente
#[derive(Clone, Copy, Default)]
pub struct LobbyPlayer {
    pub is_ready: bool,
    pub is_host: bool,
}

impl Game {
    pub fn new() -> Self {
        Self {
            state: GameState::Intro,
            intro_started_at: macroquad::time::get_time(),
            my_id: String::new(),
            connection_error: None,
            host_join_focus: HostJoinFocus::Identifier,
            is_host: true,
            is_ready: false,
            lobby_players: HashMap::new(),
            lobby_state_sent: false,
            lobby_state_last_sent: 0.0,
            input_ip: String::from("127.0.0.1"),
            host_ip: String::new(),
            is_zombie: false,
            position: [0.0, 0.0],
            movement_speed: MovementSpeed::Fast,
            movement_target: None,
            other_players: HashMap::new(),
            other_player_zombie_states: HashMap::new(),
            last_sent_position: None,
            last_sent_zombie_state: None,
            client: None,
            world: None,
            server: None,
            board: None,
            obstacles_requested: false,
        }
    }

    /// Met à jour les interactions du jeu en fonction de la scène courante
    pub fn update(&mut self, input: &Input) {
        match self.state {
            GameState::Intro => states::intro::update(self, input),
            GameState::HostJoin => states::host_join::update(self, input),
            GameState::Lobby => states::lobby::update(self, input),
            GameState::Playing => states::playing::update(self, input),
        }
    }

    /// Retourne l'état du jeu
    pub fn get_state(&self) -> GameState {
        self.state
    }

    /// Renvoie le temps écoulé (secondes) depuis le début de l'intro
    pub fn intro_elapsed(&self) -> f32 {
        (macroquad::time::get_time() - self.intro_started_at) as f32
    }

    /// Change l'état du jeu
    pub fn set_state(&mut self, state: GameState) {
        self.state = state;
    }
}

/// Entrées présentement détectées
#[derive(Default)]
pub struct Input {
    pub start: bool,
    pub enter: bool,
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub ready: bool,
    pub backspace: bool,
    pub tab: bool,
    pub character: Option<char>,
    pub escape: bool,
}
