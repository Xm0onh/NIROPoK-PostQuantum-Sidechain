use std::collections::HashMap;
// Remove unused imports from bytes
// use bytes::{Buf, BufMut, Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::iter::Iterator;

// Add these traits to fix the iter() and other method errors
use std::slice::Iter;
use std::f64::consts::E;
use bincode;

// Constants from the Go code
const MAX_REVEALS: u64 = 1024;
const MAX_PROOF_DIGESTS: u64 = 20 * MAX_REVEALS;

#[derive(Debug, Clone)]
pub struct Params {
    pub msg: Vec<u8>,
    pub proven_weight: u64,
    pub sec_kq: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    pub pk: Vec<u8>,  // Changed from signature.PublicKey to Vec<u8> for simplicity
    pub weight: u64,
}

impl Participant {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.pk.clone();
        bytes.extend_from_slice(&u64::to_le_bytes(self.weight));
        bytes
    }
}

#[derive(Debug, Clone)]
pub struct Participants(Vec<Participant>);

impl Participants {
    pub fn new(participants: Vec<Participant>) -> Self {
        Participants(participants)
    }

    pub fn to_bytes(&self) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
        self.0.iter()
            .map(|p| Ok(p.to_bytes()))
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SigSlot {
    sig: Vec<u8>,
    l: u64,
}

impl SigSlot {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.sig.clone();
        bytes.extend_from_slice(&u64::to_le_bytes(self.l));
        bytes
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reveal {
    party: Participant,
    sig_slot: SigSlot,
}

impl Reveal {
    pub fn size(&self) -> u64 {
        let party_bytes = self.party.to_bytes();
        let sig_slot_bytes = self.sig_slot.to_bytes();
        (party_bytes.len() + sig_slot_bytes.len()) as u64
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cert {
    pub sig_commit: Vec<u8>,
    pub signed_weight: u64,
    pub sig_proofs: Vec<Vec<u8>>,
    pub party_proofs: Vec<Vec<u8>>,
    pub reveals: HashMap<u64, Reveal>,
}

impl Cert {
    pub fn size(&self) -> u64 {
        let mut size = self.sig_commit.len() as u64;
        size += std::mem::size_of::<u64>() as u64;
        
        for proof in &self.sig_proofs {
            size += proof.len() as u64;
        }
        
        for proof in &self.party_proofs {
            size += proof.len() as u64;
        }
        
        for reveal in self.reveals.values() {
            size += reveal.size();
        }
        
        size
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        bincode::serialize(self).map_err(|e| e.into())
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        bincode::deserialize(bytes).map_err(|e| e.into())
    }
}

impl Params {
    // Compute the number of reveals necessary to achieve the desired security parameters
    pub fn num_reveals(&self, signed_weight: u64) -> Result<u64, String> {
        num_reveals(signed_weight, self.proven_weight, self.sec_kq, MAX_REVEALS)
    }
}

// Helper function to compute number of reveals
fn num_reveals(signed_weight: u64, proven_weight: u64, sec_kq: u64, bound: u64) -> Result<u64, String> {
    // If signed_weight is less than proven_weight, it's impossible to achieve the security parameter
    if signed_weight < proven_weight {
        return Err(format!(
            "signed weight ({}) must be greater than or equal to proven weight ({})",
            signed_weight, proven_weight
        ));
    }

    let mut n = 0u64;
    
    let sw = signed_weight as f64;
    let pw = proven_weight as f64;
    
    let mut lhs = 1.0f64;
    let mut rhs = f64::from(2.0).powf(sec_kq as f64);
    
    loop {
        if lhs >= rhs {
            return Ok(n);
        }
        
        if n >= bound {
            return Err(format!(
                "numReveals({}, {}, {}) > {}",
                signed_weight, proven_weight, sec_kq, bound
            ));
        }
        
        lhs *= sw;
        rhs *= pw;
        n += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_participant_serialization() {
        let participant = Participant {
            pk: vec![1, 2, 3, 4],
            weight: 100,
        };
        
        let bytes = participant.to_bytes();
        assert_eq!(bytes[0..4], vec![1, 2, 3, 4]);
        
        // Check if weight is correctly serialized (little-endian)
        let weight_bytes = &bytes[4..];
        assert_eq!(u64::from_le_bytes(weight_bytes.try_into().unwrap()), 100);
    }

    #[test]
    fn test_participants_collection() {
        let p1 = Participant {
            pk: vec![1, 2, 3],
            weight: 50,
        };
        let p2 = Participant {
            pk: vec![4, 5, 6],
            weight: 75,
        };
        
        let participants = Participants::new(vec![p1, p2]);
        let bytes = participants.to_bytes().unwrap();
        
        assert_eq!(bytes.len(), 2); // Two participants
        assert_eq!(bytes[0][0..3], vec![1, 2, 3]); // First participant's pk
        assert_eq!(bytes[1][0..3], vec![4, 5, 6]); // Second participant's pk
    }

    #[test]
    fn test_cert_serialization() {
        let participant = Participant {
            pk: vec![1, 2, 3],
            weight: 100,
        };
        
        let sig_slot = SigSlot {
            sig: vec![9, 8, 7],
            l: 42,
        };
        
        let reveal = Reveal {
            party: participant,
            sig_slot,
        };
        
        let mut reveals = HashMap::new();
        reveals.insert(0, reveal);
        
        let cert = Cert {
            sig_commit: vec![4, 5, 6],
            signed_weight: 150,
            sig_proofs: vec![vec![1, 1], vec![2, 2]],
            party_proofs: vec![vec![3, 3]],
            reveals,
        };
        
        // Test serialization/deserialization
        let bytes = cert.to_bytes().unwrap();
        let decoded_cert = Cert::from_bytes(&bytes).unwrap();
        
        assert_eq!(decoded_cert.sig_commit, vec![4, 5, 6]);
        assert_eq!(decoded_cert.signed_weight, 150);
        assert_eq!(decoded_cert.sig_proofs.len(), 2);
        assert_eq!(decoded_cert.party_proofs.len(), 1);
        assert_eq!(decoded_cert.reveals.len(), 1);
    }

    #[test]
    fn test_num_reveals_calculation() {
        let params = Params {
            msg: vec![1, 2, 3],
            proven_weight: 100,
            sec_kq: 10,
        };
        
        // Test with valid inputs
        let result = params.num_reveals(200);
        assert!(result.is_ok());
        
        // Test with equal weights (should require fewer reveals)
        let result = params.num_reveals(100);
        assert!(result.is_ok());
        
        // Test with smaller signed weight (should fail)
        let result = params.num_reveals(50);
        assert!(result.is_err());
    }

    #[test]
    fn test_reveal_size_calculation() {
        let participant = Participant {
            pk: vec![1, 2, 3], // 3 bytes
            weight: 100,       // 8 bytes
        };
        
        let sig_slot = SigSlot {
            sig: vec![9, 8, 7], // 3 bytes
            l: 42,              // 8 bytes
        };
        
        let reveal = Reveal {
            party: participant,
            sig_slot,
        };
        
        // Total size should be:
        // participant: 3 (pk) + 8 (weight) = 11 bytes
        // sig_slot: 3 (sig) + 8 (l) = 11 bytes
        // Total: 22 bytes
        assert_eq!(reveal.size(), 22);
    }

    #[test]
    fn test_cert_size_calculation() {
        let cert = Cert {
            sig_commit: vec![1, 2, 3],           // 3 bytes
            signed_weight: 100,                   // 8 bytes
            sig_proofs: vec![vec![1, 2], vec![3, 4]],  // 4 bytes
            party_proofs: vec![vec![5, 6]],      // 2 bytes
            reveals: HashMap::new(),             // 0 bytes (empty)
        };
        
        // Total size should be:
        // sig_commit: 3 bytes
        // signed_weight: 8 bytes
        // sig_proofs: 4 bytes
        // party_proofs: 2 bytes
        // reveals: 0 bytes
        assert_eq!(cert.size(), 17);
    }
} 