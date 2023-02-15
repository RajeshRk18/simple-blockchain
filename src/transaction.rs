use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Hash, Deserialize, PartialEq, PartialOrd, Clone)]
pub struct Txn {
    pub sender: String,
    pub receiver: String,
    pub amount: u32,
}

impl Txn {
    pub fn new(sender: String, receiver: String, amount: u32) -> Txn {
        Txn {
            sender,
            receiver,
            amount,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash)]
pub struct CoinbaseTxn {
    pub amount: u8,
    pub validator: String,
    pub message: String,
    }

impl CoinbaseTxn {
    pub fn new() -> Self {
        Self { amount: 0, validator: String::new(), message: String::new()}
        }
    }