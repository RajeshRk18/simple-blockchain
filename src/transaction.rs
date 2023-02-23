use serde::Serialize;

#[derive(Debug, Serialize, Hash, Clone)]
pub struct Txn {
    pub sender: String,
    pub receiver: String,
    pub amount: u32,
}

#[allow(dead_code)]
impl Txn {
    pub fn new(sender: String, receiver: String, amount: u32) -> Txn {
        Txn {
            sender,
            receiver,
            amount,
        }
    }
}

#[derive(Debug, Clone, Serialize, Hash)]
pub struct CoinbaseTxn {
    pub amount: u8,
    pub validator: String,
    pub message: String,
}

impl CoinbaseTxn {
    pub fn new() -> Self {
        Self {
            amount: 0,
            validator: String::new(),
            message: String::new(),
        }
    }
}

impl Default for Txn {
    fn default() -> Self {
        Self {
            sender: String::from("Anon1"),
            receiver: String::from("Anon2"),
            amount: 5,
        }
    }
}
