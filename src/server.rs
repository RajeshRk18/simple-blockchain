mod block;
mod blockchain;
mod node;
mod transaction;

use clap::Parser;
use network::receiver::MessageReceiver;
use node::Node;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::task::JoinHandle;

#[derive(Parser)]
#[clap(
    author = "Rajesh",
    version = "0.1.0",
    about = "CLI utility for nodes to respond to client requests"
)]
struct Cli {
    #[clap(short, long, value_parser, value_name = "NUM", default_value_t = 7291)]
    client_port: u16,

    #[clap(short, long, value_parser, value_name = "NUM", default_value_t = 7192)]
    server_port: u16,

    #[clap(short, long, value_parser, value_name="NUM", default_value_t=IpAddr::V4(Ipv4Addr::LOCALHOST))]
    address: IpAddr,

    #[clap(long, short, value_name = "ADDRESS")]
    boot_node: Option<SocketAddr>,
}

#[tokio::main]
async fn main() {
    simple_logger::SimpleLogger::new().env().init().unwrap();

    let cli = Cli::parse();

    let server_address = SocketAddr::new(cli.address, cli.server_port);
    let client_address = SocketAddr::new(cli.address, cli.client_port);
    let boot_node = cli.boot_node;
    let (_, network_handle, _) = spawn_tasks(server_address, client_address, boot_node).await;

    network_handle.await.unwrap();
}

async fn spawn_tasks(
    server: SocketAddr,
    client: SocketAddr,
    boot_node: Option<SocketAddr>,
) -> (JoinHandle<()>, JoinHandle<()>, JoinHandle<()>) {
    
    let (server_tcp_receiver, server_channel_receiver) = MessageReceiver::new(server);
    let server_handle = tokio::spawn(async move {
        server_tcp_receiver.run().await;
    });
    
    let (client_tcp_receiver, client_channel_receiver) = MessageReceiver::new(client);
    let client_handle = tokio::spawn(async move {
        client_tcp_receiver.run().await;
    });
    
    let mut node = Node::new(server, boot_node);
    let node_handle = tokio::spawn(async move {
        node.run(server_channel_receiver, client_channel_receiver)
            .await;
    });


    (server_handle, client_handle, node_handle)
}
