#![allow(unused_variables)]
#![allow(non_snake_case)]
#![allow(unused_imports)]

use crate::blockchain::*;
use crate::transaction::*;
use serde::{Deserialize, Serialize};
use serde_json::*;
use sha256::digest;

#[derive(Debug, Serialize, Clone, Hash)]
pub struct Body {
    pub txn_data: Vec<Txn>,
}

#[derive(Debug, Clone, Serialize, Hash)]
pub struct BlockHeader {
    pub timestamp: u64,
    pub index: u32,
    pub previous_hash: String,
    pub current_hash: String,
    pub coinbase_txn: CoinbaseTxn,
    pub merkle_root: String,
    pub nonce: u32,
    pub difficulty: u8,
}

#[derive(Debug, Clone, Serialize, Hash)]
pub struct Block {
    pub Block_header: BlockHeader,
    pub Body: Body,
}

impl Block {
    pub fn new(previous_hash: String, txn_data: Vec<Txn>) -> Block {
        let Block_header = BlockHeader {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            index: update_index(),
            previous_hash,
            current_hash: String::new(),
            coinbase_txn: CoinbaseTxn::new(),
            merkle_root: MerkleRoot::new(),
            nonce: 0,
            difficulty: update_difficulty(),
        };

        let Body = Body { txn_data };

        Block { Block_header, Body }
    }
}

#[derive(Debug)]
pub struct MerkleRoot;

impl MerkleRoot {
    pub fn new() -> String {
        String::new()
    }

    pub fn from(txns: &mut Vec<Txn>) -> String {
        if txns.is_empty() {
            digest(String::new().as_bytes());
        }

        if txns.len() % 2 != 0 {
            txns.push(txns[txns.len() - 1].clone());
        }

        let mut hashed_txns = txns
            .iter()
            .map(|txn| {
                let ser_txn = serde_json::to_string(&txn).unwrap();
                digest(ser_txn.as_bytes())
            })
            .collect::<Vec<String>>();

        let mut merkle_root = String::new();

        while hashed_txns.len() > 1 {
            let mut inner_tree: Vec<String> = hashed_txns.clone();
            let mut branches = Vec::<String>::new();
            for i in 0..inner_tree.len() / 2 {
                let index = 2 * i;
                let left = inner_tree[index].clone();
                let right = inner_tree[index + 1].clone();

                let left_right = format!("{left}{right}");

                branches.push(digest(left_right.as_bytes()));
            }

            inner_tree = branches;
            if inner_tree.len() == 1 {
                merkle_root = inner_tree[0].clone();
                break;
            }

            if inner_tree.len() % 2 != 0 {
                inner_tree.push(inner_tree[inner_tree.len() - 1].clone());
            }

            hashed_txns = inner_tree;
        }

        merkle_root
    }
}

fn update_difficulty() -> u8 {
    unsafe {
        let assign = DIFFICULTY;
        DIFFICULTY += 1;
        assign
    }
}

fn update_index() -> u32 {
    unsafe {
        let assign = BLOCK_INDEX;
        BLOCK_INDEX += 1;
        assign
    }
}
