use std::{
    collections::HashMap,
    io::{self, ErrorKind},
    net::{IpAddr, SocketAddr},
    sync::{Arc, Weak},
};

use smol::{
    Task,
    channel::{Receiver, RecvError, Sender, bounded},
    spawn,
};

use crate::{
    DEFAULT_PAYLOAD,
    pinger::{packet::IcmpMessage, socket::IcmpSocket},
};

mod packet;
mod socket;

#[derive(Debug, PartialEq, Eq, Hash)]
struct SocketIdentifier {
    is_ipv4: bool,
    source: Option<IpAddr>,
    device: Option<String>,
    netns: String,
}

struct PingerSocket {
    receiver: Receiver<(IcmpMessage, SocketAddr)>,
    task: Task<()>,
    socket: IcmpSocket,
}

pub struct Pinger {
    sockets: HashMap<SocketIdentifier, Weak<PingerSocket>>,
}

impl Pinger {
    pub fn new() -> Self {
        Self {
            sockets: HashMap::new(),
        }
    }

    pub async fn get_handle(
        &mut self,
        ip: IpAddr,
        source: Option<IpAddr>,
        device: Option<impl AsRef<str>>,
        netns: impl AsRef<str>,
    ) -> Option<PingerHandle> {
        if source.is_some_and(|source| source.is_ipv4() != ip.is_ipv4()) {
            return None;
        }

        let identifier = SocketIdentifier {
            is_ipv4: ip.is_ipv4(),
            source,
            device: device.map(|device| device.as_ref().to_string()),
            netns: netns.as_ref().to_string(),
        };

        let socket = self
            .get_socket(&identifier)
            .or_else(|| self.create_socket(identifier))?;

        Some(PingerHandle {
            receiver: socket.receiver.clone(),
            socket,
            ip: SocketAddr::new(ip, 0),
        })
    }

    fn get_socket(&self, identifier: &SocketIdentifier) -> Option<Arc<PingerSocket>> {
        self.sockets
            .get(&identifier)
            .and_then(|socket| socket.upgrade())
    }

    fn create_socket(&mut self, identifier: SocketIdentifier) -> Option<Arc<PingerSocket>> {
        let socket = IcmpSocket::new(
            identifier.is_ipv4,
            identifier.source,
            identifier.device.as_ref(),
            &identifier.netns,
        )
        .inspect_err(|error| {
            log::error!(
                "Can't create socket with [src {}] [dev {}] [ns {}]: {error}",
                identifier
                    .source
                    .as_ref()
                    .map(IpAddr::to_string)
                    .unwrap_or("none".to_string()),
                identifier
                    .device
                    .as_ref()
                    .map(String::as_str)
                    .unwrap_or("none"),
                identifier.netns
            )
        })
        .ok()?;

        let (sender, receiver) = bounded(8);
        let task = spawn(receiver_task(socket.clone(), sender));
        let socket = Arc::new(PingerSocket {
            receiver,
            task,
            socket,
        });

        self.sockets.insert(identifier, Arc::downgrade(&socket));

        Some(socket)
    }
}

async fn receiver_task(socket: IcmpSocket, sender: Sender<(IcmpMessage, SocketAddr)>) {
    let mut buffer = [0; 2048];

    loop {
        let (length, ip) = match socket.recv_from(&mut buffer).await {
            Ok(value) => value,
            Err(error) if let ErrorKind::WouldBlock | ErrorKind::TimedOut = error.kind() => {
                continue;
            }
            _ => break,
        };
        let Ok(packet) = IcmpMessage::try_from(&buffer[..length]) else {
            continue;
        };
        if let Err(_) = sender.force_send((packet, ip)) {
            break;
        }
    }
}

pub struct PingerHandle {
    receiver: Receiver<(IcmpMessage, SocketAddr)>,
    socket: Arc<PingerSocket>,
    ip: SocketAddr,
}

impl PingerHandle {
    pub async fn send(&self, identifier: u16, sequence: u16, ttl: u16) -> io::Result<()> {
        let packet = IcmpMessage::Echo {
            identifier,
            sequence,
            payload: DEFAULT_PAYLOAD.as_bytes().into(),
        };
        let packet = if self.ip.is_ipv4() {
            packet.as_v4_bytes()
        } else {
            packet.as_v6_bytes()
        };

        self.socket.socket.set_ttl(ttl as u32)?;
        self.socket.socket.send_to(&packet, &self.ip).await?;

        Ok(())
    }

    pub async fn recv(&self) -> Result<IcmpMessage, RecvError> {
        loop {
            let (packet, ip) = self.receiver.recv().await?;

            if ip == self.ip {
                break Ok(packet);
            }
        }
    }
}
