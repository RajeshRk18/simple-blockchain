/* Abstract implementation of Sender end of the channel.
Also includes receiver connection because sender end needs the task and 
so the sender spawns connection between sender and receiver */

use super::error::NetworkError::*;

use bytes::Bytes;
use std::{collections::HashMap, net::SocketAddr};
use futures::sink::SinkExt as _;
use futures::stream::StreamExt as _;
use log::{info, warn};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{self, *};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

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
            }
        } else {
            let new_sender = Self::spawn_sender(addr);
            if let Err(_) = new_sender.send(data.clone()).await {
                warn!("Failed to send message to {}", addr);
                self.connections.insert(addr, new_sender);
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
                warn!("{}", FailedToConnect(self.address, e));
                return;
            }
        };

        while let Some(data) = self.receiver.recv().await {
            if let Err(e) = writer.send(data).await {
                warn!("{}", FailedToSend(self.address, e));
            }
        }

        if let Some(Ok(_)) = reader.next().await {
            info!("Received ACK from {}", self.address);
        } else {
            warn!("{}", NoACKReceipt(self.address))
        }
    }
}

#[cfg(test)]
mod async_tests {
    use super::*;

    use bytes::Bytes;
    use futures::future::try_join_all;
    use std::net::SocketAddr;
    use tokio::net::TcpListener;
    use tokio::task::JoinHandle;
    use tokio_util::codec::{Framed, LengthDelimitedCodec};

    #[tokio::test]
    async fn test_send() {
        let address = "127.0.0.1:6100".parse::<SocketAddr>().unwrap();
        let message = "Hello, world!";

        let handle = listener(address, message.to_string());
        
        // Make the network sender and send the message.
        let mut sender = MessageSender::new();
        sender.send(address, Bytes::from(message)).await;

        // Ensure the server received the message.
        assert!(handle.await.is_ok());
    }

    #[tokio::test]
    async fn broadcast() {
        // Run 3 TCP servers.
        let message = "Hello, world!";
        let (handles, addresses): (Vec<_>, Vec<_>) = (0..3)
            .map(|x| {
                let address = format!("127.0.0.1:{}", 6_200 + x)
                    .parse::<SocketAddr>()
                    .unwrap();
                (listener(address, message.to_string()), address)
            })
            .collect::<Vec<_>>()
            .into_iter()
            .unzip();

        // Make the network sender and send the message.
        let mut sender = MessageSender::new();
        sender.broadcast(addresses, Bytes::from(message)).await;

        // Ensure all servers received the broadcast.
        assert!(try_join_all(handles).await.is_ok());
    }

    fn listener(address: SocketAddr, expected: String) -> JoinHandle<()> {
        tokio::spawn(async move {
            let listener = TcpListener::bind(&address)
                .await
                .expect("Address/port already in use");
            let (socket, _) = listener.accept().await.unwrap();
            let transport = Framed::new(socket, LengthDelimitedCodec::new());
            let (mut writer, mut reader) = transport.split();
            match reader.next().await {
                Some(Ok(received)) => {
                    assert_eq!(received, expected);
                    writer.send(Bytes::from("Ack")).await.unwrap()
                }
                _ => panic!("Failed to receive network message"),
            }
        })
    }
}
