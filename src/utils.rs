
use crate::validator::Validator;

#[derive(Debug, Clone, Copy)]
pub struct Seed {
    pub seed: [u8; 32],
}

impl Seed {
    pub fn new_epoch_seed(validator: &Validator) -> Self {
        let mut seed = vec![0u8; 32];
        for (_, hash_value) in validator.hash_chain_com.iter() {
            for (i, byte) in hash_value.hash_chain_index.as_bytes().iter().enumerate() {
                seed[i] ^= *byte;
            }
        }
        Seed { seed: seed.try_into().unwrap() }
    }

    pub fn get_seed(&self) -> [u8; 32] {
        self.seed
    }
}

pub fn get_block_seed(proposer_hash: String, prev_seed: [u8; 32]) -> [u8; 32] {
    let mut seed = vec![0u8; 32];
    for (i, byte) in proposer_hash.as_bytes().iter().enumerate() {
        seed[i] = *byte ^ prev_seed[i];
    }
    seed.try_into().unwrap()
}

// pub fn compute_validators_weight(seed: [u8; 32],  )