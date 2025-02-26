use std::net::SocketAddr;

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
