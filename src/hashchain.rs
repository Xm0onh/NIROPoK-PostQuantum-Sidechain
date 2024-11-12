use sha3::{Digest, Sha3_256};
use rand::Rng;
use crate::config::EPOCH_DURATION;
use serde::{Serialize, Deserialize};
#[derive(Debug,Serialize, Deserialize, Clone)]
pub struct HashChain {
    pub hash_chain: Vec<String>
}

#[derive(Debug,Serialize, Deserialize)]
pub struct HashChainMessage {
    pub hash_chain_index: String,
}

impl HashChain {
    pub fn new() -> Self {
        let mut hasher = Sha3_256::new();
        let nonce = rand::thread_rng().gen_range(0..u64::MAX);
        hasher.update(nonce.to_string().as_bytes());
        let mut hash_chain = vec![];
        for _ in 0..(EPOCH_DURATION + 1) {
            let hash = hasher.clone().finalize();
            hasher.update(hash);
            hash_chain.push(hex::encode(hash));
        };
        HashChain { hash_chain }
    }

    pub fn get_hash(&self, index: usize) -> HashChainMessage {
        HashChainMessage { hash_chain_index: self.hash_chain[index].clone() }
    }
}

pub fn verify_hash_chain_index(commitment: String, index: u64, received_hash: HashChainMessage) -> bool {
    let mut hasher = Sha3_256::new();
    hasher.update(received_hash.hash_chain_index.as_bytes());
    for i in index..(EPOCH_DURATION + 1) {
        let hash = hasher.clone().finalize();
        hasher.update(hash);
    }
    hex::encode(hasher.finalize()) == commitment
}