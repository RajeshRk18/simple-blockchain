use crate::block::*;
use crate::blockchain::BlockChain;
use crate::transaction::{CoinbaseTxn, Txn};
use crate::sender::MessageSender;
use anyhow::{bail, Result};
use log::{info, warn, debug};
use crate::error::NetworkError;
use rand::{thread_rng, Rng as _};
use serde::*;
use sha2::{Digest as _, Sha256};
use std::collections::HashSet;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::io::AsyncReadExt as _;

const REWARD: u8 = 50;

pub struct Mine {
    // Why task joinhandle required?
    // Because we need to have a control over the miner task.
    // For example, when we need to abort the mining task because another peer has already mined,
    // we need to abort the mining task running in the node.
    task: JoinHandle<()>,

    // Channel to send and receiver blocks.
    // Having them as field so that it can be used anywhere and need not to pass it as a function argument.
    block_sender: mpsc::Sender<Block>,
    block_receiver: mpsc::Receiver<Block>,

}

impl Mine {
    pub async fn mine(txns: Vec<Txn>, previous_block: Block) -> Block {
        let merkle_root = MerkleRoot::from(txns.clone());
        let mut block = Block::new(previous_block.block_header.current_hash.clone(), txns);
        block.block_header.merkle_root = merkle_root;
        block.block_header.nonce = thread_rng().gen::<u32>();

        let difficulty = block.block_header.difficulty as usize;
        let target: String = vec!["0"; difficulty].join("").into();

        debug!(&target);

        const YIELD_INTERVAL: u32 = 10000;
        // max iter per session to yield back to the executor who will send abort signal if the current block has been mined.
        // This will help us rerun miner task with new block and not infinitely work on mining already mined blocks.

        // This yield can be avoided by having the Node sending a mined signal using a mpsc channel.
        // As receiver end cannot be sent outside the self(which is the struct Mine) due to uncertainty in lifetime,
        // we occasionally make this thread yield occasionally to the executor.
        loop {
            if block.block_header.nonce % YIELD_INTERVAL == 0 {
                tokio::task::yield_now().await;
            }
            let block_hash = BlockChain::hash_block(block.clone());

            let hash_to_bits = block_hash.iter().fold(String::new(), |acc, byte| {
                let bits = format!("{byte:0>8b}");
                acc + bits.as_str()
            });

            if hash_to_bits.starts_with(target.as_str()) {
                dbg!(hash_to_bits);
                info!("{}", format!("Mined!⚡️"));
                block.block_header.coinbase_txn.amount = REWARD;
                block.block_header.coinbase_txn.validator =
                    format!("0x{}", thread_rng().gen::<u32>()); // TODO: Node network address should be added

                let mut hasher = Sha256::new();
                hasher.update(&serde_json::to_string(&block).unwrap().as_bytes());

                let hash = hex::encode(hasher.finalize().as_slice().to_owned());

                block.block_header.current_hash = hash;

                return block;
            }

            block.block_header.nonce += 1;
        }
    }

    pub fn mine_genesis() -> Block {
        let nonce = thread_rng().gen::<u32>();
        let block_header = BlockHeader {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            index: 0,
            previous_hash: "00000".to_string(),
            current_hash: String::new(),
            coinbase_txn: CoinbaseTxn::new(),
            merkle_root: MerkleRoot::new(),
            nonce,
            difficulty: DIFFICULTY,
        };
        let body = Body { txn_data: vec![] };

        let mut block = Block { block_header, body };

        let merkle_root = MerkleRoot::from(block.body.txn_data.clone());

        block.block_header.merkle_root = merkle_root;

        let difficulty = block.block_header.difficulty as usize;
        let target: String = vec!["0"; difficulty].join("").into();

        loop {
            let block_hash = BlockChain::hash_block(block.clone());

            let hash_to_bits = block_hash.iter().fold(String::new(), |acc, byte| {
                let bits = format!("{byte:0>8b}");
                acc + bits.as_str()
            });

            if hash_to_bits.starts_with(target.as_str()) {
                info!("{}", format!("Mined genesis!👀🎉"));
                block.block_header.coinbase_txn.amount = REWARD;
                block.block_header.coinbase_txn.validator =
                    format!("0x{}", thread_rng().gen::<u32>());

                let mut hasher = Sha256::new();
                hasher.update(&serde_json::to_string(&block).unwrap().as_bytes());

                let hash = hex::encode(hasher.finalize().as_slice().to_owned());

                block.block_header.current_hash = hash;

                return block;
            }

            block.block_header.nonce += 1;
        }
    }
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
    pub async fn new(address: SocketAddr, seed: Option<SocketAddr>) -> Result<Self> {      
        let mut peers = HashSet::<SocketAddr>::with_capacity(10);

        if let Some(seed) = seed {
            peers.insert(seed);
        }

        let (block_sender, block_receiver) = mpsc::channel::<Block>(500);

        match seed {
            Some(node) => {
                info!(
                    "Syncing with latest state of Blockchain, Seed node: {}",
                    node
                );
                let mut sender = MessageSender::new();

                let get_latest_state = Message::GetState { receiver: address };

                match bincode::serialize(&get_latest_state) {
                    Ok(bytes) => {

                        sender.send(node, bytes.into()).await;

                        let mut node_connect = TcpStream::connect(node).await.unwrap();

                        let mut response = Vec::new();

                        node_connect.read_to_end(&mut response).await?;
                        match bincode::deserialize(&response) {
                            Ok(response) => match response {
                                Message::ShareState { from, peers, state } => {
                                    info!("Received State from {}", from);
                                    return Ok(Self {
                                        address,
                                        sender,
                                        peers,
                                        mempool: HashSet::new(),
                                        state,
                                        miner: Mine {
                                            task: tokio::spawn(async {}),
                                            block_sender,
                                            block_receiver,
                                        },
                                    });
                                }
                                Message::GetState { .. } | Message::Txn { .. } => {
                                    return Err(NetworkError::BootNodeReceiveError(node).into());
                                }
                            },
                            Err(_) => {
                                return Err(NetworkError::BootNodeReceiveError(node).into());
                            }
                        }
                    }

                    Err(_) => {
                        return Err(NetworkError::DeserializeError.into());
                    }
                }
            }
            None => {
                return Ok(Self {
                    address,
                    sender: MessageSender::new(),
                    peers,
                    mempool: HashSet::new(),
                    state: BlockChain::new(),
                    miner: Mine {
                        task: tokio::spawn(async {}),
                        block_sender,
                        block_receiver,
                    },
                });
            }
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
                // Receive block from miner thread
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

                    let new_block_check_passed = new_block_root == verify_root
                        && new_block.block_header.index
                            == current_latest_block.block_header.index + 1
                        && current_latest_block.block_header.current_hash
                            == new_block.block_header.previous_hash;

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
                info!("Restarting miner thread...");
                let block = block.clone();
                let txns = self.mempool.clone().into_iter().collect();
                let block_sender = self.miner.block_sender.clone();

                self.miner.task = tokio::spawn(async move {
                    let new_block = Mine::mine(txns, block).await;
                    if let Err(e) = block_sender.send(new_block).await {
                        warn!("Can't send mined block to receiver: {}", e);
                    }
                });
            }

            None => {
                info!("Mining genesis block!");
                let block_sender = self.miner.block_sender.clone();
                self.miner.task = tokio::spawn(async move {
                    let new_block = Mine::mine_genesis();
                    if let Err(e) = block_sender.send(new_block).await {
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
