/// Messages échangés entre les clients et le serveur
pub const MAX_UDP_DATAGRAM_SIZE: usize = 65_507;

pub enum Message {
    // Net = messages de gestion réseau entre le serveur et le client
    NETGetID, // demande d'ID
    NETKeepAlive {
        id: String,
    }, // ligne de vie
    NETReturnID {
        id: String,
    }, // retour d'ID
    NETRegisterID {
        id: String,
    }, // demande de réservation d'un identifiant choisi
    NETIDTaken, // identifiant déjà utilisé

    // C2S = du client vers le serveur, S2C = du serveur vers le client
    C2SObstacles {
        id: String,
    }, // demande d'obstacles
    C2SPosition {
        id: String,
        x: usize,
        y: usize,
    }, // position
    C2SZombieState {
        id: String,
        is_zombie: bool,
    }, // état zombie d'un joueur
    C2SLobbyState {
        id: String,
        is_ready: bool,
        is_host: bool,
    },
    C2SStartGame {
        id: String,
    },
    S2CObstacles {
        obstacles: String,
    }, // obstacles
    S2CPosition {
        id: String,
        x: usize,
        y: usize,
    }, // position d'un joueur
    S2CZombieState {
        id: String,
        is_zombie: bool,
    }, // état zombie d'un joueur
    S2CLobbyPlayer {
        id: String,
        is_ready: bool,
        is_host: bool,
    },
    S2CStartGame,
    S2CZombify, // zombification du joueur
}

impl Message {
    // Sérialisation ultra-simple en texte pour éviter les dépendances lourdes (Serde)
    pub fn serialize(&self) -> Vec<u8> {
        match self {
            Message::NETGetID => "GID".to_string().into_bytes(),
            Message::NETKeepAlive { id } => format!("KEEP|{}", id).into_bytes(),
            Message::NETReturnID { id } => format!("RID|{}", id).into_bytes(),
            Message::NETRegisterID { id } => format!("REG|{}", id).into_bytes(),
            Message::NETIDTaken => "TAKEN".to_string().into_bytes(),
            Message::C2SObstacles { id } => format!("GOBS|{}", id).into_bytes(),
            Message::C2SPosition { id, x, y } => format!("1POS|{}|{}|{}", id, x, y).into_bytes(),
            Message::C2SZombieState { id, is_zombie } => {
                format!("ZSTA|{}|{}", id, is_zombie).into_bytes()
            }
            Message::C2SLobbyState {
                id,
                is_ready,
                is_host,
            } => format!("LOBBY|{}|{}|{}", id, is_ready, is_host).into_bytes(),
            Message::C2SStartGame { id } => format!("START|{}", id).into_bytes(),
            Message::S2CObstacles { obstacles } => format!("ROBS|{}", obstacles).into_bytes(),
            Message::S2CPosition { id, x, y } => format!("*POS|{}|{}|{}", id, x, y).into_bytes(),
            Message::S2CZombieState { id, is_zombie } => {
                format!("*ZSTA|{}|{}", id, is_zombie).into_bytes()
            }
            Message::S2CLobbyPlayer {
                id,
                is_ready,
                is_host,
            } => format!("*LOBBY|{}|{}|{}", id, is_ready, is_host).into_bytes(),
            Message::S2CStartGame => "GAME".to_string().into_bytes(),
            Message::S2CZombify => "ZOMB".to_string().into_bytes(),
        }
    }

    // Désérialisation des paquets reçus
    pub fn deserialize(bytes: &[u8]) -> Option<Self> {
        let text = std::str::from_utf8(bytes).ok()?;
        
        let parts: Vec<&str> = text.split('|').collect();
        if parts.is_empty() {
            return None;
        }

        match parts[0] {
            "GID" if parts.len() == 1 => Some(Message::NETGetID),
            "KEEP" if parts.len() == 2 => {
                let id = parts[1].to_string();
                Some(Message::NETKeepAlive { id })
            }
            "RID" if parts.len() == 2 => {
                let id = parts[1].to_string();
                Some(Message::NETReturnID { id })
            }
            "REG" if parts.len() == 2 => Some(Message::NETRegisterID {
                id: parts[1].to_string(),
            }),
            "TAKEN" if parts.len() == 1 => Some(Message::NETIDTaken),
            "1POS" if parts.len() == 4 => {
                let id = parts[1].to_string();
                let x = parts[2].parse().ok()?;
                let y = parts[3].parse().ok()?;
                Some(Message::C2SPosition { id, x, y })
            }
            "ZSTA" if parts.len() == 3 => {
                let id = parts[1].to_string();
                let is_zombie = parts[2].parse().ok()?;
                Some(Message::C2SZombieState { id, is_zombie })
            }
            "LOBBY" if parts.len() == 4 => Some(Message::C2SLobbyState {
                id: parts[1].to_string(),
                is_ready: parts[2].parse().ok()?,
                is_host: parts[3].parse().ok()?,
            }),
            "START" if parts.len() == 2 => Some(Message::C2SStartGame {
                id: parts[1].to_string(),
            }),
            "GOBS" if parts.len() == 2 => {
                let id = parts[1].to_string();
                Some(Message::C2SObstacles { id })
            }
            "ROBS" if parts.len() == 2 => {
                let obstacles = parts[1].to_string();
                Some(Message::S2CObstacles { obstacles })
            }
            "*POS" if parts.len() == 4 => {
                let id = parts[1].to_string();
                let x = parts[2].parse().ok()?;
                let y = parts[3].parse().ok()?;
                Some(Message::S2CPosition { id, x, y })
            }
            "*ZSTA" if parts.len() == 3 => {
                let id = parts[1].to_string();
                let is_zombie = parts[2].parse().ok()?;
                Some(Message::S2CZombieState { id, is_zombie })
            }
            "*LOBBY" if parts.len() == 4 => Some(Message::S2CLobbyPlayer {
                id: parts[1].to_string(),
                is_ready: parts[2].parse().ok()?,
                is_host: parts[3].parse().ok()?,
            }),
            "GAME" if parts.len() == 1 => Some(Message::S2CStartGame),
            "ZOMB" if parts.len() == 1 => Some(Message::S2CZombify),
            _ => None,
        }
    }
}

/// Vérifie si un caractère est valide pour un identifiant de joueur
pub fn is_valid_player_id_character(character: char) -> bool {
    character.is_ascii_alphanumeric()
        || matches!(character, ' ' | '!' | '-' | '.' | '?' | '&' | ',' | '\'')
}

/// Vérifie si un identifiant de joueur est valide
pub fn is_valid_player_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 32 && id.chars().all(is_valid_player_id_character)
}
