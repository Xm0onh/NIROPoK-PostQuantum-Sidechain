use crate::utils::Seed;
use crate::config::EPOCH_DURATION;
pub struct Epoch {
    pub timestamp: u64,
}


impl Epoch {
    pub fn new() -> Self {
        Self { 
            timestamp: 0 
        }
    }
    
    pub fn progress(&mut self) {
        self.timestamp += 1;
    }

    pub fn reset(&mut self) {
        self.timestamp = 0;
    }

    pub fn is_end_of_epoch(&self) -> bool {
        self.timestamp >= EPOCH_DURATION
    }
}
