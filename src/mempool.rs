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

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.transactions.clear();
    }
}
