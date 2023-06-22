mod transaction;

use anyhow::Result;
use clap::Parser;
use log::{info, warn};
use network::sender::MessageSender;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[derive(Parser)]
#[clap(
    author = "Rajesh",
    version = "0.1.0",
    about = "CLI utility to send transaction to the node"
)]
struct Cli {
    #[clap(subcommand)]
    command: Command,

    #[clap(long, short, value_parser, value_name = "NUM", default_value_t = 1729)]
    port: u16,

    #[clap(long, short, value_parser, value_name="NUM", default_value_t=IpAddr::V4(Ipv4Addr::LOCALHOST))]
    address: IpAddr,
}

#[derive(Parser, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Command {
    Txn {
        sender: String,
        receiver: String,
        value: u32,
    },
}

impl Command {
    pub async fn request(self, address: SocketAddr) -> Result<()> {
        let mut sender = MessageSender::new();
        let txn = bincode::serialize(&self)?;
        let response = sender.send(address, txn.into()).await;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    simple_logger::SimpleLogger::new().env().init()?;

    let address = SocketAddr::new(cli.address, cli.port);

    match cli.command.request(address).await {
        Ok(()) => info!("Sent transaction to Node: {}", address),
        Err(e) => warn!("Failed to send transaction: {:?}", e),
    }

    Ok(())
}
