use crate::block::Block;
use crate::mempool::Mempool;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};
use crate::wallet::Wallet;
use crate::validator::Validator;
pub struct Blockchain {
    pub chain: Vec<Block>,
    pub mempool: Mempool,
    pub wallet: Wallet,
    pub validator: Validator,
}

impl Blockchain {
    pub fn new(wallet: Wallet) -> Self {
        Self {
            chain: vec![],
            mempool: Mempool::new(),
            wallet,
            validator: Validator::new(),
        }
    }
}
