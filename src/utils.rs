use crate::accounts::Account;
use crate::validator::Validator;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::time::Instant;

// --- TPS Tracker Struct ---
#[derive(Debug)]
pub struct TpsTracker {
    pub start_time: Instant,
    pub total_transactions_confirmed: u64,
}
// --- End TPS Tracker Struct ---

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Seed {
    pub seed: [u8; 32],
}

impl Seed {
    pub fn new_epoch_seed(validator: &Validator) -> Self {
        let mut seed = vec![0u8; 32];
        for (_, hash_value) in validator.hash_chain_com.iter() {
            for (i, byte) in hash_value.hash_chain_index.as_bytes().iter().enumerate() {
                if i < 32 {
                    seed[i] ^= *byte;
                }
            }
        }
        Seed {
            seed: seed.try_into().unwrap(),
        }
    }

    pub fn get_seed(&self) -> [u8; 32] {
        self.seed
    }
}

pub fn get_block_seed(proposer_hash: String, prev_seed: [u8; 32]) -> Seed {
    let mut seed = vec![0u8; 32];
    for (i, byte) in proposer_hash.as_bytes().iter().enumerate() {
        if i < 32 {
            seed[i] = *byte ^ prev_seed[i];
        }
    }
    Seed {
        seed: seed.try_into().unwrap(),
    }
}

pub fn select_block_proposer(seed: Seed, validator: &Validator) -> &Account {
    let n: f64 = 1e9;
    let mut weights = vec![0f64; validator.state.accounts.len()];
    let mut proposer = &validator.state.accounts[0];

    for (i, account) in validator.state.accounts.iter().enumerate() {
        let mut hasher = Sha3_256::new();
        hasher.update(seed.get_seed());
        // validator hash chain commitment
        // info!("Validator hash chain commitment: {:?}", validator.hash_chain_com.get(&account.address).unwrap().hash_chain_index);
        // info!("Validator latest epoch hash: {:?}", validator.next_block_hash.get(&account));
        // let mut hash_value = &validator.hash_chain_com.get(&account.address).unwrap().hash_chain_index;
        // if validator.next_block_hash.get(&account).is_some() {
        //     hash_value = validator.next_block_hash.get(&account).unwrap();
        // }
        if let Some(hash_value) = validator.hash_chain_com.get(&account.address) {
            hasher.update(hash_value.hash_chain_index.as_bytes());
            let hash_result = hasher.finalize();
            let numeric_value = u64::from_be_bytes([
                hash_result[0],
                hash_result[1],
                hash_result[2],
                hash_result[3],
                hash_result[4],
                hash_result[5],
                hash_result[6],
                hash_result[7],
            ]);

            if let Some(balance) = validator.state.balances.get(&account) {
                weights[i] = n - (numeric_value as f64 / balance);
            }
        }
    }

    let mut lowest_weight = f64::INFINITY;
    for (i, weight) in weights.iter().enumerate() {
        if *weight < lowest_weight {
            lowest_weight = *weight;
            proposer = &validator.state.accounts[i];
        }
    }
    proposer
}
