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

        let difficulty: usize = block.Block_header.difficulty.clone() as usize;
        let expected_slice = vec![48u8; difficulty]; // Assumed 1 difficulty represents 1byte. So using ASCII Code for zero.
        let txns = serde_json::to_string::<Vec<Txn>>(block.Body.txn_data.as_ref()).unwrap();
        dbg!(&expected_slice);
        loop {
            let prev_hash = block.Block_header.previous_hash.clone();

            let hash_gen_format = format!("{}{}{}", block.Block_header.nonce, txns, prev_hash);
            let hash_gen = digest(hash_gen_format);

            if hash_gen.split_at(difficulty).0.as_bytes()[0..difficulty] == expected_slice {
                dbg!(hash_gen.split_at(difficulty).0.as_bytes()[0..difficulty].as_ref());
                dbg!(expected_slice);
                block.Block_header.coinbase_txn.amount = REWARD;
                block.Block_header.coinbase_txn.validator =
                    format!("0x{}", thread_rng().gen::<u32>().to_string());
                block.Block_header.coinbase_txn.message = format!(
                    "Mined by {}",
                    block.Block_header.coinbase_txn.validator.clone()
                );

                block.Block_header.current_hash = digest(serde_json::to_string(&block).unwrap());
                dbg!(&block);
                self.blocks.push(block);
                break;
            }

            block.Block_header.nonce += 1;
        }
    }
}
