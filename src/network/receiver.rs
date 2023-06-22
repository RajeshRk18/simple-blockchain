// Abstract implementation of Receiver end of the channel
use super::error::NetworkError::*;
use anyhow::Result;
use bytes::Bytes;
use futures::{stream::SplitSink, SinkExt as _, StreamExt as _};
use log::{info, warn};
use serde::{de::DeserializeOwned, Serialize};
use std::{fmt::Debug, net::SocketAddr};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, oneshot},
};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

pub struct MessageReceiver<Request, Response> {
    address: SocketAddr,
    sender: mpsc::Sender<(Request, oneshot::Sender<Response>)>,
}

impl<Request, Response> MessageReceiver<Request, Response>
where
    Request: DeserializeOwned + Sync + Send + Debug + 'static,
    Response: Serialize + Send + Debug + 'static,
{
    pub fn new(addr: SocketAddr) -> (Self, mpsc::Receiver<(Request, oneshot::Sender<Response>)>) {
        let (sender, receiver) = mpsc::channel(500);
        (
            Self {
                address: addr,
                sender,
            },
            receiver,
        )
    }

    pub async fn run(&self) {
        let listener = TcpListener::bind(self.address).await.unwrap();

        info!("Listening to {}", self.address);

        loop {
            let (stream, sender) = match listener.accept().await {
                Ok(accepted) => accepted,
                Err(e) => {
                    warn!("{}", FailedToReceive(self.address, e));
                    continue;
                }
            };

            info!("Incoming connection established with {}", sender);
            Self::spawn(stream, sender, self.sender.clone()).await;
        }
    }

    async fn spawn(
        stream: TcpStream,
        sender: SocketAddr,
        channel: mpsc::Sender<(Request, oneshot::Sender<Response>)>,
    ) {
        tokio::spawn(async move {
            let (mut writer, mut reader) = Framed::new(stream, LengthDelimitedCodec::new()).split();

            while let Some(message) = reader.next().await {
                match message.map_err(|e| FailedToReceive(sender, e)) {
                    Ok(message) => {
                        let msg = message.into();
                        if let Err(e) = Self::dispatch(&mut writer, channel.clone(), msg).await {
                            warn!("Failed to dispatch message {}", e);
                        }
                    },

                    Err(e) => {
                        warn!("{}", e);
                        return;
                    },
                }
            }
        });
    }

    async fn dispatch(
        writer: &mut SplitSink<Framed<TcpStream, LengthDelimitedCodec>, Bytes>,
        sender: mpsc::Sender<(Request, oneshot::Sender<Response>)>,
        message: Bytes,
    ) -> Result<()> {
        let request = bincode::deserialize(&message)?;

        let (response_sender, response_receiver) = oneshot::channel();

        sender.send((request, response_sender)).await?;

        let response = response_receiver.await?;

        let response = bincode::serialize(&response)?;

        writer.send(response.into()).await?;

        Ok(())
    }
}
