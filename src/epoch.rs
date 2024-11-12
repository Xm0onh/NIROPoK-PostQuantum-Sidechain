use crate::utils::Seed;
use crate::validator::Validator;
pub struct Epoch {
    pub seed: Seed,
    pub timestamp: u64,
}


impl Epoch {
    pub fn new(validator: &Validator) -> Self {
        Self { 
            seed: Seed::new_epoch_seed(validator), 
            timestamp: 0 
        }
    }

    pub fn get_seed(&self) -> Seed {
        self.seed
    }

    pub fn reset(&mut self) {
        self.timestamp = 0;
    }
}