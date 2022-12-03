fn main() {
    //Reward for validating a block
    const REWARD: u8= 50;

    //Structure of a blockchain
    struct BlockChain {
        blocks: Vec<Block>,
    }

    //implementation for blockchain
    impl BlockChain {
        fn new() -> BlockChain {
            BlockChain { blocks: vec![] }
        }
    }
    #[derive(Debug)]
    //Defining the structure of a block
    struct Block{
        block_header: String,
        block_no: u32,
        previous_hash: String,
        txn_data: Vec<TxnType>,
        nonce: u64,
        difficulty: u64,
    }

    //implementation for block
    impl Block{
        fn new(block_header: String,
            block_no: u32,
            previous_hash: String,
            txn_data: Vec<TxnType>,
            nonce: u64,
            difficulty: u64) -> Block{

            Block {
                block_header,
                block_no,
                previous_hash,
                txn_data,
                nonce,
                difficulty,
            }
        }
    }

    fn add_block(block: Block, blockchain: &mut BlockChain) ->&mut BlockChain{

        blockchain.blocks.push(block);

        blockchain
    }

    //Transaction data
    // Two variants of a transaction: normal and coinbase
    #[derive(Debug)]
    struct CoinbaseTxn {
        amount: u8,
        validator: String,
    }

    //implementing coinbase txn
    impl CoinbaseTxn{
        fn new(amount: u8, validator: String) -> CoinbaseTxn{
            CoinbaseTxn {
                amount,
                validator,
            }
        }
    }

    #[derive(Debug)]
    struct Txn{
        sender: String,
        receiver: String,
        amount: u32,
        gas_fees: f32,
    }

    //implementing txn

    impl Txn{
        fn new(sender: String, receiver: String, amount: u32, gas_fees: f32) -> Txn{
            Txn {
                sender,
                receiver,
                amount,
                gas_fees,
            }
        }
    }
    #[derive(Debug)]
    enum TxnType {
        Txn(Txn),
        CoinbaseTxn(CoinbaseTxn)
    }

    fn hash(block: &Block)->String{
        use sha256::digest;
        let input = format!("{}{}{:?}{:?}{}",block.block_header, block.block_no, block.txn_data, block.previous_hash, block.nonce);
        
        digest(input)
    
    }

    //New blockchain instance
    let mut blockchain = BlockChain::new();

    // block0
    let previous_hash = String::from("00000000000000000000000");
    let mut txns: Vec<TxnType> = vec![];    
    let coinbase_txn = CoinbaseTxn{amount: REWARD, validator: "Validator1".to_string()};

    txns.push(TxnType::CoinbaseTxn(coinbase_txn));

    let block = Block::new("Block0".to_string(), 0, previous_hash, txns, 0, 0);
    let previous_hash = hash(&block);    
    add_block(block, &mut blockchain);

    // block1
    let mut txns: Vec<TxnType> = vec![];
    let txn1 = Txn::new("Anon1".to_string(), "Anon2".to_string(), 10, 0.05);
    let txn2 = Txn::new("Anon2".to_string(), "Anon3".to_string(), 8, 0.04);
    let txn3 = Txn::new("Anon3".to_string(),"Anon1".to_string(), 5, 0.02);
    let coinbase_txn = CoinbaseTxn::new(50, "Validator2".to_string());

    txns.push(TxnType::Txn(txn1));
    txns.push(TxnType::Txn(txn2));
    txns.push(TxnType::Txn(txn3));
    txns.push(TxnType::CoinbaseTxn(coinbase_txn));

    let block = Block::new("Block0".to_string(), 0, previous_hash, txns, 0, 0);
    let current_hash = hash(&block);    
    add_block(block, &mut blockchain);

    // block2

    println!("{}", &current_hash);

    }       
