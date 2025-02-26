use crate::block::*;
use crate::transaction::*;

use anyhow::Result;
use log::error;
use serde::Deserialize;
use serde::Serialize;
use sha2::{Digest as _, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockChain {
    pub blocks: Vec<Block>,
}

impl BlockChain {
    pub fn new() -> Self {
        Self { blocks: vec![] }
    }

    pub fn tip(&self) -> String {
        self.blocks
            .last()
            .unwrap()
            .block_header
            .current_hash
            .clone()
    }

    pub fn all_blocks_in_longest_chain(&self) -> Vec<Block> {
        self.blocks.clone()
    }

    pub fn add_block(&mut self, new_block: Block) -> Result<Self> {
        match self.blocks.last() {
            Some(previous_block) => {
                if new_block.block_header.previous_hash != previous_block.block_header.current_hash
                {
                    error!("Block is an invalid extension of the previous blockchain state");
                }
                self.blocks.push(new_block);
                Ok(self.clone())
            }
            None => {
                self.blocks.push(new_block);
                Ok(self.clone())
            }
        }
    }

    pub fn hash_block(block: Block) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(&block.block_header.index.to_string().as_bytes());
        hasher.update(&block.block_header.previous_hash.as_bytes());
        hasher.update(&block.block_header.difficulty.to_string().as_bytes());
        hasher.update(&block.block_header.timestamp.to_string().as_bytes());
        hasher.update(&block.block_header.nonce.to_string().as_bytes());
        hasher.update(Self::hash_txn_batch(&block.body.txn_data).as_bytes());
        let hash = hasher.finalize().as_slice().to_owned();
        hash
    }

    pub fn hash_txn_batch(txns: &Vec<Txn>) -> String {
        let mut hasher = Sha256::new();
        for txn in txns {
            let txn_hash = Self::hash_txn(txn);
            if txns.len() == 1 {
                return txn_hash;
            }
            hasher.update(&txn_hash.as_bytes());
        }

        let hash = hasher.finalize().as_slice().to_owned();
        hex::encode(hash)
    }

    pub fn hash_txn(txn: &Txn) -> String {
        let mut hasher = Sha256::new();
        hasher.update(txn.id.as_bytes());
        hasher.update(txn.sender.as_bytes());
        hasher.update(txn.receiver.as_bytes());
        hasher.update(txn.amount.to_string().as_bytes());
        let hash = hasher.finalize().as_slice().to_owned();
        let hex_string = hex::encode(hash);
        hex_string
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
