use sha3::{Digest, Sha3_256};
use rand::Rng;
use crate::config::EPOCH_DURATION;
use serde::{Serialize, Deserialize};
use log::info;
#[derive(Debug,Serialize, Deserialize, Clone)]
pub struct HashChain {
    pub hash_chain: Vec<String>
}

#[derive(Debug,Serialize, Deserialize, Clone)]
pub struct HashChainMessage {
    pub hash_chain_index: String,
}

impl HashChain {
    pub fn new() -> Self {
        let mut hasher = Sha3_256::new();
        let nonce = rand::thread_rng().gen_range(0..u64::MAX);
        hasher.update(nonce.to_string().as_bytes());
        let mut hash_chain = vec![];
        for i in 0..(EPOCH_DURATION + 1) {
            let hash = hasher.clone().finalize();
            hasher.update(hash);
            hash_chain.push(hex::encode(hash));
            info!("Computed hash: {:?}, index {:?}", hex::encode(hash), i);
        };
        HashChain { hash_chain }
    }

    pub fn get_hash(&self, index: usize) -> HashChainMessage {
        HashChainMessage { hash_chain_index: self.hash_chain[index].clone() }
    }
}

pub fn verify_hash_chain_index(commitment: String, index: u64, received_hash: &HashChainMessage) -> bool {
    info!("Verifying hash chain index: {:?} at index {:?}", received_hash.hash_chain_index, EPOCH_DURATION - index + 1);
    let mut hasher = Sha3_256::new();
    hasher.update(received_hash.hash_chain_index.as_bytes());
    for _ in (EPOCH_DURATION - index + 1)..(EPOCH_DURATION + 1) {
        let hash = hasher.clone().finalize();
        hasher.update(hash);
    }
    info!("Computed hash: {:?}", hex::encode(hasher.clone().finalize()));
    hex::encode(hasher.finalize()) == commitment
}