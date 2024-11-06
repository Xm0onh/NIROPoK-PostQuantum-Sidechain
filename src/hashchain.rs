use sha3::{Digest, Sha3_256};
use rand::Rng;
use crate::constant::EPOCH_DURATION;
use serde::{Serialize, Deserialize};
#[derive(Debug,Serialize, Deserialize)]
pub struct HashChain {
    pub hash_chain: Vec<String>
}

impl HashChain {
    pub fn new() -> Self {
        let mut hasher = Sha3_256::new();
        let nonce = rand::thread_rng().gen_range(0..u64::MAX);
        hasher.update(nonce.to_string().as_bytes());
        let mut hash_chain = vec![];
        for _ in 0..EPOCH_DURATION {
            let hash = hasher.clone().finalize();
            hasher.update(hash);
            hash_chain.push(hex::encode(hash));
        };
        HashChain { hash_chain }
    }

    pub fn get_hash(&self, index: usize) -> String {
        self.hash_chain[index].clone()
    }
}
