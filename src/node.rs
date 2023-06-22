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

    State {
        from: SocketAddr,
        peers: HashSet<SocketAddr>,
        state: BlockChain,
    },
}

pub struct Node {
    address: SocketAddr,
    sender: MessageSender,
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
            // Receive block from miner task
            if let Some(block) = self.miner.block_receiver.recv().await {
                info!("Block received from Miner task: {:?}", block);

                if let Ok(new_state) = self.state.extend(block) {
                    self.update_state(new_state).await;
                }
            }

            // Receive transaction request from client
            if let Some((client_request, response_sender)) = client_handle.recv().await {
                info!("Received txn request from client: {:?}", client_request);
                let result = self
                    .handle_message(client_request.clone().try_into().unwrap())
                    .await
                    .map_err(|e| e.to_string());

                if let Err(e) = response_sender.send(result) {
                    warn!("Failed to send message {:?}", e);
                }
            }

            // Receive message from peer
            if let Some((message, reply_sender)) = peer_handle.recv().await {
                info!("Received peer message {:?}", message);
                reply_sender.send("Acknowledged".to_string()).unwrap();
                self.handle_message(message.clone()).await.unwrap();
            }
        }
    }

    pub async fn handle_message(&mut self, message: Message) -> Result<Option<String>> {
        match message {
            Message::GetState { receiver } => {
                self.peers.insert(receiver);
                let response = Message::State {
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

            Message::State { from, peers, state } => {
                self.peers.insert(from);
                self.peers.extend(peers);
                self.peers.remove(&self.address);

                if state.blocks.len() > self.state.blocks.len() {
                    info!("Received longest chain from {}", from);

                    let new_block = state.blocks.last().unwrap();
                    let new_block_root = new_block.block_header.merkle_root.clone();
                    let verify_root = MerkleRoot::from(new_block.body.txn_data.clone());

                    if new_block_root == verify_root {
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

        self.run_miner();

        let state = Message::State {
            from: self.address,
            peers: self.peers.clone(),
            state: self.state.clone(),
        };

        self.broadcast(state).await;
    }

    fn run_miner(&mut self) {
        match self.state.blocks.last() {

            Some(block) => {
                info!("Restarting miner task...");
                let block = block.clone();
                let txns = self.mempool.clone().into_iter().collect();
                let sender = self.miner.block_sender.clone();
                self.miner.task.abort();
        
                self.miner.task = tokio::spawn(async move {
                    let new_block = BlockChain::mine(txns, block).await;
                    if let Err(e) = sender.send(new_block).await {
                        warn!("Can't send mined block to receiver channel: {}", e);
                    }
                });
            },

            None => {
                info!("Mining genesis block!");
                let sender = self.miner.block_sender.clone();
                self.miner.task = tokio::spawn(async move {
                    let new_block = BlockChain::mine_genesis().await;
                    if let Err(e) = sender.send(new_block).await {
                        warn!("Can't send mined block to receiver channel: {}", e);
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
