use crate::block::*;
use crate::blockchain::BlockChain;
use crate::transaction::Txn;
use ::network::sender::MessageSender;
use anyhow::{bail, Result};
use log::{info, warn};
use serde::*;
use std::collections::HashSet;
use std::net::SocketAddr;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

pub struct Mine {
    task: JoinHandle<()>,
    block_sender: mpsc::Sender<Block>,
    block_receiver: mpsc::Receiver<Block>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    Txn {
        id: String,
        sender: String,
        receiver: String,
        amount: u32,
    },

    GetState {
        receiver: SocketAddr,
    },

    ShareState {
        from: SocketAddr,
        peers: HashSet<SocketAddr>,
        state: BlockChain,
    },
}

pub struct Node {
    address: SocketAddr,
    sender: MessageSender, // Receiver end of the channel is embedded in MessageSender.
    peers: HashSet<SocketAddr>,
    mempool: HashSet<Txn>,
    state: BlockChain,
    miner: Mine,
}

impl Node {
    pub fn new(address: SocketAddr, seed: Option<SocketAddr>) -> Self {
        let mut peers = HashSet::<SocketAddr>::with_capacity(10);

        if let Some(seed) = seed {
            peers.insert(seed);
        }

        let (sender, receiver) = mpsc::channel(500);

        Self {
            address,
            sender: MessageSender::new(),
            peers,
            mempool: HashSet::new(),
            state: BlockChain::new(),
            miner: Mine {
                task: tokio::spawn(async {}),
                block_sender: sender,
                block_receiver: receiver,
            },
        }
    }

    pub async fn run(
        &mut self,
        mut peer_handle: mpsc::Receiver<(Message, oneshot::Sender<String>)>,
        mut client_handle: mpsc::Receiver<(Txn, oneshot::Sender<Result<Option<String>, String>>)>,
    ) -> JoinHandle<()> {

        self.run_miner();

        let state_message = Message::GetState {
            receiver: self.address,
        };
        self.broadcast(state_message).await;

        loop {
            tokio::select! {
                // Receive block from miner task
                Some(block) = self.miner.block_receiver.recv() => {
                    info!("Block received from Miner task: {:?}", block);
    
                    if let Ok(new_state) = self.state.add_block(block) {
                        info!("Updating state");
                        self.update_state(new_state).await;
                    }
                }
    
                // Receive transaction request from client
                Some((client_request, node)) = client_handle.recv() => {
                    info!("Received txn request from client: {:?}", client_request);
                    let result = self
                        .handle_message(client_request.clone().try_into().unwrap())
                        .await
                        .map_err(|e| e.to_string());
    
                    if let Err(e) = node.send(result) {
                        warn!("Failed to send response {:?}", e);
                    }
                }
    
                // Receive message from peer
                Some((message, node)) = peer_handle.recv() => {
                    info!("Received peer message {:?}", message);
                    node.send("Acknowledged".to_string()).unwrap();
                    self.handle_message(message.clone()).await.unwrap();
                }
            }
        }
    }

    pub async fn handle_message(&mut self, message: Message) -> Result<Option<String>> {
        match message {
            Message::GetState { receiver } => {
                self.peers.insert(receiver);
                let response = Message::ShareState {
                    from: self.address,
                    peers: self.peers.clone(),
                    state: self.state.clone(),
                };
                let data = match bincode::serialize(&response).map_err(|e| e.to_string()) {
                    Ok(data) => data,
                    Err(e) => {
                        bail!(
                            "Failed to serialize message: {:?}\nError: {:?}",
                            response.clone(),
                            e
                        );
                    }
                };
                self.sender.send(receiver, data.into()).await;
            }

            Message::ShareState { from, peers, state } => {
                self.peers.insert(from);
                self.peers.extend(peers);
                self.peers.remove(&self.address);

                if state.blocks.len() > self.state.blocks.len() {
                    info!("Received longest chain from {}", from);

                    let current_latest_block = self.state.blocks.last().unwrap();
                    let new_block = state.blocks.last().unwrap();
                    let new_block_root = new_block.block_header.merkle_root.clone();
                    let verify_root = MerkleRoot::from(new_block.body.txn_data.clone());

                    let new_block_check_passed = new_block_root == verify_root && new_block.block_header.index == current_latest_block.block_header.index + 1 && current_latest_block.block_header.current_hash == new_block.block_header.previous_hash;
                    if new_block_check_passed {
                        self.update_state(state).await;
                    } else {
                        warn!("Not a valid state transition {}", state);
                    }
                }
            }

            Message::Txn { .. } => {
                let txn = message.clone();
                if self.mempool.insert(Txn::try_from(txn).unwrap()) {
                    self.broadcast(message).await;
                    return Ok(Some("Transaction processed".to_string()));
                }
            }
        }

    Ok(None)
    }

    async fn update_state(&mut self, new_state: BlockChain) {
        self.state = new_state;

        self.mempool.retain(|txn| {
            !self
                .state
                .blocks
                .last()
                .unwrap()
                .body
                .txn_data
                .contains(txn)
        });

        let state = Message::ShareState {
            from: self.address,
            peers: self.peers.clone(),
            state: self.state.clone(),
        };

        self.broadcast(state).await;
        self.stop_and_restart().await;
    }

    async fn stop_and_restart(&mut self) {
        self.miner.task.abort();
        self.run_miner();
    }

    fn run_miner(&mut self) {

        match self.state.blocks.last() {

            Some(block) => {
                info!("Restarting miner task...");
                let block = block.clone();
                let txns = self.mempool.clone().into_iter().collect();
                let block_sender = self.miner.block_sender.clone();
                
                self.miner.task = tokio::spawn(async move {
                    let new_block = BlockChain::mine(txns, block).await;
                    if let Err(e) = block_sender.send(new_block).await {
                        warn!("Can't send mined block to receiver: {}", e);
                    }
                });
            },

            None => {
                info!("Mining genesis block!");
                let signal_receiver = self.miner.block_sender.clone();
                self.miner.task = tokio::spawn(async move {
                    let new_block = BlockChain::mine_genesis();
                    if let Err(e) = signal_receiver.send(new_block).await {
                        warn!("Can't send mined block to receiver: {}", e);
                    }
                });
            }
        }
    }

    async fn broadcast(&mut self, message: Message) {
        let data = match bincode::serialize(&message).map_err(|e| e.to_string()) {
            Ok(data) => data,
            Err(e) => {
                warn!("Failed to serialize the message: {:?}", e);
                return;
            }
        };

        let peers = self.peers.clone().into_iter().collect();

        info!("Broadcasting to {:?}", peers);

        self.sender.broadcast(peers, data.into()).await;
    }
}

impl TryFrom<Txn> for Message {
    type Error = String;
    fn try_from(value: Txn) -> __private::Result<Self, Self::Error> {
        Ok(Message::Txn {
            id: value.id,
            sender: value.sender,
            receiver: value.receiver,
            amount: value.amount,
        })
    }
}

impl Into<Txn> for Message {
    fn into(self) -> Txn {
        match self {
            Message::Txn {
                id,
                sender,
                receiver,
                amount,
            } => Txn::with_id(id, sender, receiver, amount),
            _ => unreachable!(),
        }
    }
}
