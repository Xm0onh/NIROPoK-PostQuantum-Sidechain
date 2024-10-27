use crate::block::Block;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

pub struct Blockchain {
    pub chain: Vec<Block>,
}