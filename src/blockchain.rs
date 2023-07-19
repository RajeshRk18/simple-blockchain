use crate::block::*;
use crate::transaction::*;

use anyhow::{bail, Result};
use rand::{thread_rng, Rng as _};
use serde::Deserialize;
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use log::info;

pub const DIFFICULTY: u8 = 2;
pub static mut BLOCK_INDEX: u32 = 0;

const REWARD: u8 = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockChain {
    pub blocks: Vec<Block>,
}

impl BlockChain {
    pub fn new() -> BlockChain {
        BlockChain { blocks: vec![] }
    }

    pub async fn mine(txns: Vec<Txn>, previous_block: Block) -> Block {
        let merkle_root = MerkleRoot::from(txns.clone());
        let mut block = Block::new(previous_block.block_header.current_hash.clone(), txns);
        block.block_header.merkle_root = merkle_root;
        block.block_header.nonce = thread_rng().gen::<u32>();

        let difficulty = block.block_header.difficulty as usize;
        let target: String = vec!["0"; difficulty].join("").into();

        const YIELD_INTERVAL: u32 = 10000;
    
        loop {
            if block.block_header.nonce % YIELD_INTERVAL == 0 {
                tokio::task::yield_now().await;
            }

            let block_hash = Self::hash(block.clone());

            let hash_to_bits = block_hash
                .iter()
                .fold(String::new(), |acc, byte| {
                    let bits = format!("{byte:0>8b}");
                    acc + bits.as_str()
                });

            if hash_to_bits.starts_with(target.as_str()) {
                info!("{}", format!("Mined!⚡️"));
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
        let body = Body {txn_data: vec![]};

        let mut block = Block {
            block_header,
            body
        };

        let merkle_root = MerkleRoot::from(block.body.txn_data.clone());

        block.block_header.merkle_root = merkle_root;

        let difficulty = block.block_header.difficulty as usize;
        let target: String = vec!["0"; difficulty].join("").into();

        loop {

            let block_hash = Self::hash(block.clone());

            let hash_to_bits = block_hash
                .iter()
                .fold(String::new(), |acc, byte| {
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

    pub fn extend(&self, new_block: Block) -> Result<Self> {
        match self.blocks.last() {
            Some(previous_block) => {
                if new_block.block_header.previous_hash != previous_block.block_header.current_hash
                {
                    bail!("Block is an invalid extension of the previous blockchain state");
                }
                let mut new_chain = self.clone();
                new_chain.blocks.push(new_block);
                Ok(new_chain)
            },
            None => {
                let mut new_chain = self.clone();
                new_chain.blocks.push(new_block);
                Ok(new_chain)                
            }
        }
    }

    pub fn hash(block: Block) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(&block.block_header.index.to_string().as_bytes());
        hasher.update(&block.block_header.previous_hash.as_bytes());
        hasher.update(&block.block_header.difficulty.to_string().as_bytes());
        hasher.update(&block.block_header.timestamp.to_string().as_bytes());
        hasher.update(&block.block_header.nonce.to_string().as_bytes());
        hasher.update(Self::hash_txns(&block.body.txn_data).as_bytes());
        let hash = hasher.finalize().as_slice().to_owned();
        hash
    }

    pub fn hash_txns(txns: &Vec<Txn>) -> String {
        let mut hasher = Sha256::new();
        for txn in txns {
            let mut inner_hasher = Sha256::new();
            inner_hasher.update(&txn.id.as_bytes());
            inner_hasher.update(&txn.sender.as_bytes());
            inner_hasher.update(&txn.receiver.as_bytes());
            inner_hasher.update(&txn.amount.to_string().as_bytes());

            let hash = inner_hasher.finalize().as_slice().to_owned();
            let hash = hex::encode(hash);
            if txns.len() == 1 {
                return hash;
            }
            hasher.update(&hash.as_bytes());
        }

        let hash = hasher.finalize().as_slice().to_owned();
        hex::encode(hash)
    }
}

impl std::fmt::Display for BlockChain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Current State:\nLatest Block: {:?}",
            self.blocks.last().unwrap()
        )
    }
}
