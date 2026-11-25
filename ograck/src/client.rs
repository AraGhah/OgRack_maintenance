/// Client UDP simple
///
/// Il maintient un identifiant unique qui lui est attribué par le serveur lors de la
/// connexion.
///
/// Le client envoie régulièrement des messages de "keep-alive" pour informer le serveur
/// qu'il est toujours actif. Si le serveur ne reçoit pas de message de "keep-alive" d'un
/// client pendant un certain temps, il considère que le client est inactif et le supprime
/// de sa liste.
///
/// Les communications réseau sont gérées dans un thread séparé, ce qui permet au
/// client de rester réactif et de ne pas bloquer l'exécution principale.
///
///     thread principal                  thread (worker)
///    ┌───────────────┐                 ┌───────────────┐
///    │               │-----------------│               │
///    │       tx ---------- commands --------> rx       │
///    │               │-----------------│               │
///    │               │                 │               │
///    │               │-----------------│               │
///    │       rx <---- received_messages ----- tx       │
///    │               │-----------------│               │
///    └───────────────┘                 └───────────────┘
///
use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::message::{MAX_UDP_DATAGRAM_SIZE, Message, is_valid_player_id};
use crate::server::SERVER_PORT;

const CLIENT_TICK: Duration = Duration::from_millis(100);
const CLIENT_ID_TIMEOUT: Duration = Duration::from_secs(5);

enum ClientCommand {
    Send(Vec<u8>),
    Shutdown,
}

pub struct Client {
    pub client_id: String,
    commands: Sender<ClientCommand>,
    received_messages: Receiver<Message>,
    worker: Option<JoinHandle<io::Result<()>>>,
}

impl Client {
    /// Établit une connexion avec le serveur
    pub fn connect(server_address: &str, requested_id: Option<&str>) -> Result<Self, String> {
        if let Some(id) = requested_id
            && !is_valid_player_id(id)
        {
            return Err("Identifiant invalide".to_string());
        }
        
        let full_address = if server_address.contains(':') {
            server_address.to_string()
        } else {
            format!("{server_address}:{SERVER_PORT}")
        };
        
        let server_address = full_address
            .parse::<SocketAddr>()
            .map_err(|_| "Adresse IP : format invalide".to_string())?;
        
        let socket = UdpSocket::bind("0.0.0.0:0")
            .map_err(|error| format!("Échec de liaison du client: {error}"))?;
        socket
            .send_to(
                &match requested_id {
                    Some(id) => Message::NETRegisterID { id: id.to_string() },
                    None => Message::NETGetID,
                }
                .serialize(),
                server_address,
            )
            .map_err(|error| format!("Échec de requête de l'identifiant client: {error}"))?;

        let deadline = Instant::now() + CLIENT_ID_TIMEOUT;
        let mut buffer = [0u8; MAX_UDP_DATAGRAM_SIZE];

        let client_id = loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err("Expiration de l'attente de l'identifiant client".to_string());
            }

            socket
                .set_read_timeout(Some(remaining))
                .map_err(|error| format!("Échec de configuration du client: {error}"))?;

            match socket.recv_from(&mut buffer) {
                Ok((amount, address)) if address == server_address => {
                    if let Some(Message::NETReturnID { id }) =
                        Message::deserialize(&buffer[..amount])
                    {
                        if requested_id.is_some_and(|requested_id| requested_id != id) {
                            return Err("Mauvais identifiant retourné".to_string());
                        }
                        break id;
                    }
                    if matches!(
                        Message::deserialize(&buffer[..amount]),
                        Some(Message::NETIDTaken)
                    ) {
                        return Err("Identifiant indisponible".to_string());
                    }
                }
                Ok(_) => {}
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                    ) =>
                {
                    return Err("Identifiant non reçu dans les délais".to_string());
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(_) => {
                    return Err("Échec de réception de l'identifiant".to_string());
                }
            }
        };

        socket
            .set_read_timeout(Some(CLIENT_TICK))
            .map_err(|error| format!("Échec de configuration du client: {}", error))?;

        let (command_tx, command_rx) = mpsc::channel();
        let (received_tx, received_rx) = mpsc::channel();
        
        let worker_client_id = client_id.clone();

        let worker = thread::spawn(move || {
            run(
                socket,
                server_address,
                worker_client_id,
                command_rx,
                received_tx,
            )
        });

        Ok(Self {
            client_id,
            commands: command_tx,
            received_messages: received_rx,
            worker: Some(worker),
        })
    }

    /// Envoie un message au serveur
    pub fn send(&self, message: &Message) -> io::Result<()> {
        self.commands
            .send(ClientCommand::Send(message.serialize()))
            .map_err(|error| io::Error::new(io::ErrorKind::BrokenPipe, error))
    }

    /// Récupère un message reçu du serveur.
    /// 
    /// Cette méthode est non-bloquante et retourne Ok(None) s'il n'y a pas de message.
    pub fn receive(&self) -> io::Result<Option<Message>> {
        match self.received_messages.try_recv() {
            Ok(message) => Ok(Some(message)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "Canal de réception client déconnecté",
            )),
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.commands.send(ClientCommand::Shutdown);

        if let Some(worker) = self.worker.take() {
            match worker.join() {
                Ok(Ok(())) => {}
                Ok(Err(error)) => eprintln!("Arrêt du client avec une erreur: {error}"),
                Err(_) => eprintln!("Panique du fil d'exécution client"),
            }
        }
    }
}

/// Fil d'exécution du côté client de la communication réseau
fn run(
    socket: UdpSocket,
    server_address: SocketAddr,
    client_id: String,
    commands: Receiver<ClientCommand>,
    received_messages: Sender<Message>,
) -> io::Result<()> {
    let mut buffer = [0u8; MAX_UDP_DATAGRAM_SIZE];

    loop {
        loop {
            match commands.try_recv() {
                Ok(ClientCommand::Send(bytes)) => {
                    socket.send_to(&bytes, server_address)?;
                }
                Ok(ClientCommand::Shutdown) | Err(TryRecvError::Disconnected) => return Ok(()),
                Err(TryRecvError::Empty) => break,
            }
        }

        match socket.recv_from(&mut buffer) {
            Ok((amount, _)) => {
                if let Some(message) = Message::deserialize(&buffer[..amount])
                    && received_messages.send(message).is_err()
                {
                    return Ok(());
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                let keep_alive = Message::NETKeepAlive {
                    id: client_id.clone(),
                };
                socket.send_to(&keep_alive.serialize(), server_address)?;
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
}

/// Obtient l'adresse IP locale effective du client
///
/// La façon la plus robuste de le faire est d'établir une connexion UDP vers
/// un serveur externe (comme le DNS de Google) et de lire l'adresse locale
/// utilisée pour cette connexion (l'adresse IP de l'interface réseau utilisée).  
pub fn get_local_ip() -> Option<String> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;

    socket.connect("8.8.8.8:80").ok()?;

    Some(socket.local_addr().ok()?.ip().to_string())
}
