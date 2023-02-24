use crate::block::*;
use crate::transaction::*;

use rand::{thread_rng, Rng};
use serde::Serialize;
use sha256::digest;

// DIFFICULTY LVL & INDEX as mut coz both change in every iteration of pow and adding block respectively
pub static mut DIFFICULTY: u8 = 1;
pub static mut BLOCK_INDEX: u32 = 0;

const REWARD: u8 = 50;
#[derive(Debug, Serialize)]
pub struct BlockChain {
    pub blocks: Vec<Block>,
}

impl BlockChain {
    pub fn new() -> BlockChain {
        BlockChain { blocks: vec![] }
    }

    pub fn add_block(&mut self, mut block: Block) {
        let merkle_root = MerkleRoot::from(&mut block.Body.txn_data);

        block.Block_header.merkle_root = merkle_root;

        let difficulty: usize = block.Block_header.difficulty as usize;
        let expected_slice = vec![0u8; difficulty]
            .iter()
            .fold(String::new(), |acc, bit| acc + bit.to_string().as_str());
        let txns = serde_json::to_string::<Vec<Txn>>(block.Body.txn_data.as_ref()).unwrap();

        let prev_hash = block.Block_header.previous_hash.clone();
        loop {
            let str_format = format!("{}{}{}", block.Block_header.nonce, txns, prev_hash);
            let hash_gen = digest(str_format);
            let bit_serialized = hash_gen.as_bytes().iter().fold(String::new(), |acc, byte| {
                let bits = format!("{byte:0>8b}");
                acc + bits.as_str()
            });

            if bit_serialized.split_at(difficulty).0 == expected_slice {
                block.Block_header.coinbase_txn.amount = REWARD;
                block.Block_header.coinbase_txn.validator =
                    format!("0x{}", thread_rng().gen::<u32>());
                block.Block_header.coinbase_txn.message = format!(
                    "Mined by {}",
                    block.Block_header.coinbase_txn.validator.clone()
                );

                block.Block_header.current_hash = digest(serde_json::to_string(&block).unwrap());
                self.blocks.push(block);
                break;
            }

            block.Block_header.nonce += 1;
        }
    }
}
