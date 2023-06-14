mod block;
mod blockchain;
mod transaction;

use block::*;
use blockchain::*;
use transaction::*;

fn main() {
    //New blockchain instance
    let mut blockchain = BlockChain::new();

    println!("-----------------------------------------------\n   ---------------BLOCKCHAIN----------------");

    // block0
    let previous_hash = String::from("00000000");

    blockchain.add_block(Block::new(previous_hash, vec![]));

    println!("{:#?}", &blockchain);
    

    loop {

        let previous_hash = blockchain
            .blocks
            .last()
            .unwrap()
            .Block_header
            .current_hash
            .clone();

        blockchain.add_block(Block::new(
            previous_hash,
            vec![Txn::default(), Txn::default(), Txn::default()],
        ));

        println!("{:#?}", &blockchain);
    }
}
