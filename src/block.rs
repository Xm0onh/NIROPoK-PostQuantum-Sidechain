use crate::transaction::Transaction;
use serde::{Serialize, Deserialize};
use rs_merkle::{Hasher, MerkleTree};
use sha3::{Digest, Sha3_256};


#[derive(Clone)]
struct Sha3Hasher;

impl Hasher for Sha3Hasher {
    type Hash = [u8; 32];
    fn hash(data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha3_256::new();
        hasher.update(data);
        hasher.finalize().into()
    }
}

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

    fn compute_merkle_root(&self) -> [u8; 32] {
        let leaves: Vec<[u8; 32]> = self.txn
        .iter()
        .map(|tx| tx.hash.clone())
        .collect();
        let tree = MerkleTree::<Sha3Hasher>::from_leaves(&leaves);
        tree.root().unwrap()
    }
}
