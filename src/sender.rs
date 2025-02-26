/* Abstract implementation of Sender end of the channel.
Also includes receiver connection because sender end creates receiver on demand */

use bytes::Bytes;
use futures::sink::SinkExt as _;
use futures::stream::StreamExt as _;
use log::{info, warn};
use std::{collections::HashMap, net::SocketAddr};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{self, *};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Failed to connect to {0}\n{1}")]
    FailedToConnect(SocketAddr, std::io::Error),

    #[error("Failed to send message to {0}.\n{1}")]
    FailedToSend(SocketAddr, std::io::Error),

    #[error("Failed to receive message from {0}")]
    FailedToReceive(SocketAddr, std::io::Error),

    #[error("Failed to receive ACK from {0}.")]
    NoACKReceipt(SocketAddr),

    #[error("Received unexpected ACK from {0}.")]
    UnexpectedACK(SocketAddr),

    #[error("Failed to receive state from boot node {0}")]
    BootNodeReceiveError(SocketAddr),

    #[error("Failed to deserialize message")]
    DeserializeError,
}

/// Each peer connection is given a separate thread
#[derive(Clone)]
pub struct MessageSender {
    connections: HashMap<SocketAddr, Sender<Bytes>>,
}

impl std::default::Default for MessageSender {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageSender {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
        }
    }

    pub fn spawn_sender(addr: SocketAddr) -> Sender<Bytes> {
        let (sender, receiver) = mpsc::channel::<Bytes>(500);
        ReceiverConnection::spawn(addr, receiver);
        sender
    }

    pub async fn send(&mut self, addr: SocketAddr, data: Bytes) {
        if let Some(sender) = self.connections.get(&addr) {
            if let Err(_) = sender.send(data.clone()).await {
                warn!("Failed to send message to {}", addr);
                return;
            }
        } else {
            let sender = Self::spawn_sender(addr);
            self.connections.insert(addr, sender.clone());

            if let Err(_) = sender.send(data.clone()).await {
                warn!("Failed to send message to {}", addr);
                return;
            }
        }
    }

    pub async fn broadcast(&mut self, addresses: Vec<SocketAddr>, data: Bytes) {
        for address in addresses {
            self.send(address, data.clone()).await;
        }
    }
}

struct ReceiverConnection {
    address: SocketAddr,
    receiver: Receiver<Bytes>,
}

impl ReceiverConnection {
    pub fn spawn(address: SocketAddr, receiver: Receiver<Bytes>) {
        tokio::spawn(async move {
            Self { address, receiver }.run().await;
        });
    }

    pub async fn run(&mut self) {
        let (mut writer, mut reader) = match TcpStream::connect(self.address).await {
            Ok(stream) => Framed::new(stream, LengthDelimitedCodec::new()).split(),
            Err(e) => {
                warn!("{}", NetworkError::FailedToConnect(self.address, e));
                return;
            }
        };

        loop {
            tokio::select! {
                Some(data) = self.receiver.recv() => {
                    if let Err(e) = writer.send(data).await {
                        warn!("{:#?}", NetworkError::FailedToSend(self.address, e));
                    }
                }
    
                Some(response) = reader.next() => {
                    match response {
                        Ok(_) => info!("Received ACK from {}", self.address),
                        Err(e) => { 
                            warn!("{}", NetworkError::NoACKReceipt(self.address));
                        }
                    }
                }
            }
        }
    }
}
