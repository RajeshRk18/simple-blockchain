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
        /*let mut txns: Vec<Txn> = vec![];
        let txn1 = Txn::new("Anon1".to_string(), "Anon2".to_string(), 10);
        let txn2 = Txn::new("Anon2".to_string(), "Anon3".to_string(), 8);
        let txn3 = Txn::new("Anon3".to_string(), "Anon1".to_string(), 5);

        txns.push(txn1);
        txns.push(txn2);
        txns.push(txn3);*/

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
