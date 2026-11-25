///
/// Serveur UDP simple
///
/// Le serveur maintient une liste des clients connectés et l'horodatage de la
/// dernière communication de chacun. Il peut ainsi supprimer les clients inactifs en
/// fonction d'un délai d'expiration.
///
/// Le serveur peut envoyer des messages à tous les clients ou à un client spécifique.
/// L'interface exposée au réseau repose sur des messages sérialisés définis dans le
/// module "message". L'interface exposée aux utilisateurs du serveur est encapsulée
/// dans la structure "Server" et utilise l'énumération Message comme protocole de
/// communication.
///
/// Les communications réseau sont gérées dans un thread séparé, ce qui permet au
/// serveur de rester réactif et de ne pas bloquer l'exécution principale.
///
///     thread principal                  thread (worker)
///    ┌───────────────┐                 ┌───────────────┐
///    │               |-----------------│               │
///    │       tx ---------- commands --------> rx       │
///    │               │-----------------│               │
///    │               │                 │               │
///    │               │-----------------│               │
///    │       rx <---- received_messages ----- tx       │
///    │               │-----------------│               │
///    └───────────────┘                 └───────────────┘
///
use std::collections::HashMap;
use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::message::{MAX_UDP_DATAGRAM_SIZE, Message, is_valid_player_id};

pub const SERVER_PORT: u16 = 50505;
pub const KEEPALIVE_TIMEOUT: Duration = Duration::from_millis(500);

const SERVER_TICK: Duration = Duration::from_millis(100);
const CLIENT_ID_COUNT: usize = 1_000;

enum ServerCommand {
    SendToAllExcept(String, Vec<u8>),
    SendToOne(String, Vec<u8>),
    Shutdown,
}

struct ClientInfo {
    address: SocketAddr,
    last_seen: Instant,
}

pub struct Server {
    commands: Sender<ServerCommand>,
    received_messages: Mutex<Receiver<Message>>,
    worker: Option<JoinHandle<io::Result<()>>>,
}

impl Server {
    /// Démarre le serveur
    pub fn start() -> io::Result<Self> {
        let socket = UdpSocket::bind(format!("0.0.0.0:{SERVER_PORT}"))?;
        socket.set_read_timeout(Some(SERVER_TICK))?;

        let (command_tx, command_rx) = mpsc::channel();
        let (received_tx, received_rx) = mpsc::channel();
        let worker = thread::spawn(move || run(socket, command_rx, received_tx));

        Ok(Self {
            commands: command_tx,
            received_messages: Mutex::new(received_rx),
            worker: Some(worker),
        })
    }

    /// Envoie un message à tous les clients sauf un
    pub fn send_to_all_except(
        &self,
        excluded_client_id: &str,
        message: &Message,
    ) -> io::Result<()> {
        self.commands
            .send(ServerCommand::SendToAllExcept(
                excluded_client_id.to_owned(),
                message.serialize(),
            ))
            .map_err(|error| io::Error::new(io::ErrorKind::BrokenPipe, error))
    }

    /// Envoie un message à un client
    pub fn send_to_one(&self, client_id: &str, message: &Message) -> io::Result<()> {
        self.commands
            .send(ServerCommand::SendToOne(
                client_id.to_owned(),
                message.serialize(),
            ))
            .map_err(|error| io::Error::new(io::ErrorKind::BrokenPipe, error))
    }

    /// Récupère un message reçu d'un client
    /// 
    /// Cette méthode est non-bloquante et retourne Ok(None) s'il n'y a pas de message.
    pub fn receive(&self) -> io::Result<Option<Message>> {
        let received_messages = self
            .received_messages
            .lock()
            .map_err(|_| io::Error::other("Server receive lock is poisoned"))?;
        
        match received_messages.try_recv() {
            Ok(message) => Ok(Some(message)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "Server receive channel is disconnected",
            )),
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.commands.send(ServerCommand::Shutdown);

        if let Some(worker) = self.worker.take() {
            match worker.join() {
                Ok(Ok(())) => {}
                Ok(Err(error)) => eprintln!("Arrêt du serveur, erreur: {error}"),
                Err(_) => eprintln!("Panique du fil d'exécution serveur"),
            }
        }
    }
}

/// Fil d'exécution du côté serveur de la communication réseau
fn run(
    socket: UdpSocket,
    commands: Receiver<ServerCommand>,
    received_messages: Sender<Message>,
) -> io::Result<()> {
    let mut clients: HashMap<String, ClientInfo> = HashMap::new();
    let mut buffer = [0u8; MAX_UDP_DATAGRAM_SIZE];

    loop {
        loop {
            match commands.try_recv() {
                Ok(ServerCommand::SendToAllExcept(excluded_client_id, bytes)) => {
                    prune_expired_clients(&mut clients);
                    for (client_id, client) in &clients {
                        if client_id == &excluded_client_id {
                            continue;
                        }

                        if let Err(error) = socket.send_to(&bytes, client.address) {
                            eprintln!(
                                "Échec de communication avec le client {}: {error}",
                                client.address
                            );
                        }
                    }
                }
                Ok(ServerCommand::SendToOne(client_id, bytes)) => {
                    prune_expired_clients(&mut clients);
                    if let Some(client) = clients.get(&client_id)
                        && let Err(error) = socket.send_to(&bytes, client.address)
                    {
                        eprintln!(
                            "Échec de communication avec le client {}: {error}",
                            client.address
                        );
                    }
                }
                Ok(ServerCommand::Shutdown) | Err(TryRecvError::Disconnected) => return Ok(()),
                Err(TryRecvError::Empty) => break,
            }
        }

        match socket.recv_from(&mut buffer) {
            Ok((amount, address)) => {
                if let Some(message) = Message::deserialize(&buffer[..amount]) {
                    match message {
                        Message::NETKeepAlive { id } => {
                            clients.insert(
                                id,
                                ClientInfo {
                                    address,
                                    last_seen: Instant::now(),
                                },
                            );
                        }
                        Message::NETGetID => {
                            if let Some(id) = generate_client_id(&clients) {
                                clients.insert(
                                    id.clone(),
                                    ClientInfo {
                                        address,
                                        last_seen: Instant::now(),
                                    },
                                );

                                let response = Message::NETReturnID { id };
                                socket.send_to(&response.serialize(), address)?;
                            } else {
                                eprintln!("Identifiant client indisponible");
                            }
                        }
                        Message::NETRegisterID { id } => {
                            prune_expired_clients(&mut clients);

                            if can_register_client_id(&clients, &id) {
                                clients.insert(
                                    id.clone(),
                                    ClientInfo {
                                        address,
                                        last_seen: Instant::now(),
                                    },
                                );

                                socket
                                    .send_to(&Message::NETReturnID { id }.serialize(), address)?;
                            } else {
                                socket.send_to(&Message::NETIDTaken.serialize(), address)?;
                            }
                        }
                        Message::C2SObstacles { ref id }
                            if clients
                                .get(id)
                                .is_some_and(|client| client.address == address) =>
                        {
                            received_messages.send(message).map_err(|error| {
                                io::Error::new(io::ErrorKind::BrokenPipe, error)
                            })?;
                        }
                        Message::C2SPosition { ref id, .. }
                            if clients
                                .get(id)
                                .is_some_and(|client| client.address == address) =>
                        {
                            received_messages.send(message).map_err(|error| {
                                io::Error::new(io::ErrorKind::BrokenPipe, error)
                            })?;
                        }
                        Message::C2SZombieState { ref id, .. }
                            if clients
                                .get(id)
                                .is_some_and(|client| client.address == address) =>
                        {
                            received_messages.send(message).map_err(|error| {
                                io::Error::new(io::ErrorKind::BrokenPipe, error)
                            })?;
                        }
                        Message::C2SLobbyState { ref id, .. }
                        | Message::C2SStartGame { ref id }
                            if clients
                                .get(id)
                                .is_some_and(|client| client.address == address) =>
                        {
                            received_messages.send(message).map_err(|error| {
                                io::Error::new(io::ErrorKind::BrokenPipe, error)
                            })?;
                        }
                        _ => {}
                    }
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock
                        | io::ErrorKind::TimedOut
                        | io::ErrorKind::Interrupted
                ) => {}
            Err(error) => return Err(error),
        }

        prune_expired_clients(&mut clients);
    }
}

/// Génère un identifiant client unique de trois chiffres
fn generate_client_id(clients: &HashMap<String, ClientInfo>) -> Option<String> {
    let available_ids: Vec<usize> = (0..CLIENT_ID_COUNT)
        .filter(|number| !clients.contains_key(&format!("{number:03}")))
        .collect();

    if available_ids.is_empty() {
        return None;
    }

    let index = rand::random_range(0..available_ids.len());
    Some(format!("{:03}", available_ids[index]))
}

/// Retire les clients inactifs
fn prune_expired_clients(clients: &mut HashMap<String, ClientInfo>) {
    let now = Instant::now();
    clients.retain(|_, client| now.duration_since(client.last_seen) < KEEPALIVE_TIMEOUT);
}

/// Vérifie si un identifiant peut être accepté
fn can_register_client_id(clients: &HashMap<String, ClientInfo>, id: &str) -> bool {
    is_valid_player_id(id) && !clients.contains_key(id)
}
