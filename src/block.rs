use crate::transaction::Transaction;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub id: usize,
    pub hash: String,
    pub previous_hash: String,
    pub timestamp: usize,
    pub txn: Vec<Transaction>,
}

impl Block {
    pub fn new(id: usize, previous_hash: String, timestamp: usize, txn: Vec<Transaction>) -> Self {
        Self { id, hash: String::new(), previous_hash, timestamp, txn }
    }

    pub fn get_block_hash_by_index(&self) -> String {
        serde_json::to_string(&self.hash).unwrap()
    }

    pub fn verify_block(&self, prev_block: Block) -> Result<bool, String> {
        let previous_block_hash = prev_block.get_block_hash_by_index();
        if previous_block_hash != self.previous_hash {
            return Err("Previous block hash does not match".to_string());
        }
        /*
        - Verify the transactions
        - Verify the proof of stake
         */
        Ok(true)
    }
}
