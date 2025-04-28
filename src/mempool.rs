use crate::transaction::Transaction;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mempool {
    pub transactions: Vec<Transaction>,
}

impl Mempool {
    pub fn new() -> Self {
        Mempool {
            transactions: Vec::new(),
        }
    }

    pub fn add_transaction(&mut self, txn: Transaction) {
        self.transactions.push(txn);
    }

    pub fn delete_transaction(&mut self, txn: Transaction) {
        self.transactions.retain(|t| t.hash != txn.hash);
    }

    #[allow(dead_code)]
    pub fn get_mempool(&self) -> Vec<Transaction> {
        self.transactions.clone()
    }

    pub fn txn_exists(&self, hash: &[u8; 32]) -> bool {
        self.transactions.iter().any(|t| t.hash == *hash)
    }

    /// Retrieves up to `limit` transactions from the mempool and removes them.
    pub fn get_transactions(&mut self, limit: usize) -> Vec<Transaction> {
        // Determine how many transactions to take (up to the limit or the total available)
        let count = std::cmp::min(limit, self.transactions.len());
        // Drain the transactions from the end of the vector (can also take from the start if preferred)
        self.transactions.split_off(self.transactions.len() - count)
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.transactions.clear();
    }
}
