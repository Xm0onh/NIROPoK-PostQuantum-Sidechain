use crate::block::Block;
use crate::mempool::Mempool;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blockchain {
    pub chain: Vec<Block>,
    pub mempool: Mempool,
}