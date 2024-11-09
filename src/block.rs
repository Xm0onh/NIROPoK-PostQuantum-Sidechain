use crate::transaction::Transaction;
use serde::{Serialize, Deserialize};
use rs_merkle::{Hasher, MerkleTree};
use sha3::{Digest, Sha3_256};


#[derive(Clone)]
#[allow(dead_code)]
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
    pub hash: [u8; 32],
    pub previous_hash: [u8; 32],
    pub timestamp: usize,
    pub txn: Vec<Transaction>,
}

impl Block {
    pub fn new(id: usize, previous_hash: [u8; 32], timestamp: usize, txn: Vec<Transaction>) -> Result<Self, String> {
        let mut block = Self { 
            id,
            hash: [0u8; 32],
            previous_hash,
            timestamp,
            txn
        };
        block.hash = block.compute_merkle_root();
        Ok(block)
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
