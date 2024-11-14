use sha3::{Digest, Sha3_256};
use rand::Rng;
use crate::config::EPOCH_DURATION;
use crate::accounts::Account;
use serde::{Serialize, Deserialize};
use log::info;
#[derive(Debug,Serialize, Deserialize, Clone)]
pub struct HashChain {
    pub hash_chain: Vec<String>
}

#[derive(Debug,Serialize, Deserialize, Clone)]
pub struct HashChainCom {
    pub hash_chain_index: String,
    pub sender: Account
}

#[derive(Debug,Serialize, Deserialize, Clone)]
pub struct HashChainMessage {
    pub hash: String,
    pub sender: Account,
    pub epoch: usize
}

impl HashChain {
    pub fn new() -> Self {
        let mut hasher = Sha3_256::new();
        let mut hash_chain = vec![];

        let nonce = rand::thread_rng().gen_range(0..u64::MAX);
        hasher.update(nonce.to_be_bytes());
       
        let initial_hash = hasher.finalize();
        let initial_hash_hex: String = hex::encode(&initial_hash);
        hash_chain.push(initial_hash_hex);
       
        for i in 0..EPOCH_DURATION + 1 {
            let last_hash_bytes = hex::decode(hash_chain.last().unwrap()).expect("Invalid hex string");

            // Create a new hasher and hash the last hash
            let mut hasher = Sha3_256::new();
            hasher.update(&last_hash_bytes);
            let current_hash = hasher.finalize();
            let current_hash_hex = hex::encode(&current_hash);
            println!(
                "Computed hash: {}, index {}",
                current_hash_hex,
                i
            );
            hash_chain.push(current_hash_hex);
        };
        HashChain { hash_chain }
    }

    pub fn get_hash(&self, index: usize, sender: Account) -> HashChainCom {
        HashChainCom { hash_chain_index: self.hash_chain[index].clone(), sender: sender }
    }
}

pub fn verify_hash_chain_index(
    commitment: String,
    index: u64,
    received_hash: String,
) -> bool {
    // Decode the received hash from hex to bytes
    let mut current_hash_bytes = hex::decode(&received_hash).expect("Invalid hex string");

    // Hash the received hash (EPOCH_DURATION - index) times
    for _ in (EPOCH_DURATION - index + 1)..(EPOCH_DURATION + 1) {
        let mut hasher = Sha3_256::new();
        hasher.update(&current_hash_bytes);
        current_hash_bytes = hasher.finalize().to_vec();
    }

    let computed_commitment = hex::encode(&current_hash_bytes);
    computed_commitment == commitment
}