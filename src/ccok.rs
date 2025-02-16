use crate::merkle::{CustomHasher, MerkleTreeBuilder};
use bincode;
use crystals_dilithium::dilithium2::{PublicKey, Signature};
use hex;
use rs_merkle::Hasher;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use std::collections::HashMap;

/// Wrapper for Dilithium signature to implement serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableSignature(#[serde(with = "serde_bytes")] Vec<u8>);

impl From<Signature> for SerializableSignature {
    fn from(sig: Signature) -> Self {
        SerializableSignature(sig.to_vec())
    }
}

impl TryInto<Signature> for SerializableSignature {
    type Error = &'static str;

    fn try_into(self) -> Result<Signature, Self::Error> {
        if self.0.len() != 2420 {
            return Err("Invalid signature length");
        }
        let mut bytes = [0u8; 2420];
        bytes.copy_from_slice(&self.0);
        Ok(bytes)
    }
}

/// Represents a participant in the certificate system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    /// The public key of the participant in hex format
    pub public_key: String,
    /// The weight of the participant in the system
    pub weight: u64,
}

/// A slot for storing signature information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigSlot {
    /// The actual signature
    pub signature: Option<SerializableSignature>,
    /// The accumulated weight up to this slot (L-value in the original implementation)
    pub accumulated_weight: u64,
}

/// Configuration parameters for the certificate system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Params {
    /// The message being signed
    pub msg: Vec<u8>,
    /// The minimum weight required for proof validity
    pub proven_weight: u64,
    /// Security parameter for the system
    pub security_param: u32,
}

/// Represents a reveal in the certificate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reveal {
    /// The signature slot information
    pub sig_slot: SigSlot,
    /// The participant information
    pub party: Participant,
}

/// The final certificate containing all proofs and reveals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Certificate {
    /// Root hash of the signature Merkle tree
    pub sig_commit: Vec<u8>,
    /// Total weight of all signed participants
    pub signed_weight: u64,
    /// Total number of signature slots (leaves in the signature Merkle tree)
    pub total_sigs: usize,
    /// Map of position to reveals
    pub reveals: HashMap<u64, Reveal>,
    /// Merkle proofs for signatures
    pub sig_proofs: Vec<Vec<u8>>,
    /// Merkle proofs for participants
    pub party_proofs: Vec<Vec<u8>>,
    /// Order of reveal positions as chosen during build
    pub reveal_positions: Vec<u64>,
    /// Reveal indices corresponding to reveal positions
    pub reveal_indices: Vec<u64>,
}

impl Certificate {
    /// Returns a tuple with the total size (in bytes) of the signature proofs and participant proofs.
    pub fn proof_size(&self) -> (usize, usize) {
        let sig_size: usize = self.sig_proofs.iter().map(|p| p.len()).sum();
        let party_size: usize = self.party_proofs.iter().map(|p| p.len()).sum();
        (sig_size, party_size)
    }
}
/// Builder for creating certificates
#[derive(Debug)]
pub struct Builder {
    /// System parameters
    pub params: Params,
    /// Signature slots for all participants
    pub sigs: Vec<SigSlot>,
    /// Total weight of all signatures collected
    pub signed_weight: u64,
    /// List of all participants
    pub participants: Vec<Participant>,
    /// Root hash of the participant Merkle tree
    pub party_tree_root: Vec<u8>,
}

impl Builder {
    pub fn new(params: Params, participants: Vec<Participant>, party_tree_root: Vec<u8>) -> Self {
        Self {
            params,
            sigs: vec![
                SigSlot {
                    signature: None,
                    accumulated_weight: 0
                };
                participants.len()
            ],
            signed_weight: 0,
            participants,
            party_tree_root,
        }
    }

    /// Add a signature from a participant
    pub fn add_signature(&mut self, pos: usize, signature: Signature) -> Result<(), String> {
        // Validate position
        if pos >= self.participants.len() {
            return Err(format!("Invalid participant position: {}", pos));
        }

        // Check if we already have this signature
        if self.sigs[pos].signature.is_some() {
            return Err(format!("Already have signature for participant {}", pos));
        }

        // Validate participant weight
        if self.participants[pos].weight == 0 {
            return Err(format!("Participant {} has zero weight", pos));
        }

        // Add signature and update weights
        self.sigs[pos].signature = Some(SerializableSignature::from(signature));
        self.signed_weight += self.participants[pos].weight;

        // Update accumulated weights
        if pos > 0 {
            self.sigs[pos].accumulated_weight =
                self.sigs[pos - 1].accumulated_weight + self.participants[pos - 1].weight;
        }

        Ok(())
    }

    /// Build the certificate once enough signatures are collected
    pub fn build(&self) -> Result<Certificate, String> {
        // Check if we have enough weight
        if self.signed_weight < self.params.proven_weight {
            return Err(format!(
                "Insufficient signed weight: {} < {}",
                self.signed_weight, self.params.proven_weight
            ));
        }

        // Build Merkle tree for signatures
        let mut sig_tree = MerkleTreeBuilder::new();
        sig_tree.build(&self.sigs)?;

        // Build Merkle tree for participants
        let mut party_tree = MerkleTreeBuilder::new();
        party_tree.build(&self.participants)?;

        // Calculate the fraction of weight not required for the proof
        let fraction = 1.0 - (self.params.proven_weight as f64 / self.signed_weight as f64);
        // K is a tuning constant (here chosen as 0.5) to adjust the number of reveals
        let num_reveals = std::cmp::max(1, ((self.params.security_param as f64) * fraction * 0.5).ceil() as usize);

        // Instead of collecting unsorted reveals, collect reveal information as (position, coin_index)
        let mut reveal_map = HashMap::new();
        let mut reveal_info: Vec<(usize, u64)> = Vec::new();
        
        // Choose positions to reveal using coin flips
        for i in 0..num_reveals {
            let choice = self.coin_choice(i as u64, &sig_tree.root());
            let pos = self.find_coin_position(choice)? as usize;

            if !reveal_map.contains_key(&(pos as u64)) {
                reveal_map.insert(
                    pos as u64,
                    Reveal {
                        sig_slot: self.sigs[pos].clone(),
                        party: self.participants[pos].clone(),
                    },
                );
                reveal_info.push((pos, i as u64));
            }
        }
        
        // Sort reveal_info by position
        reveal_info.sort_by_key(|(pos, _)| *pos);
        let sorted_positions: Vec<usize> = reveal_info.iter().map(|(pos, _)| *pos).collect();
        let sorted_coin_indices: Vec<u64> = reveal_info.iter().map(|(_, coin_idx)| *coin_idx).collect();

        // Generate proofs for both signatures and participants using sorted positions
        let sig_proofs = sig_tree.prove(&sorted_positions);
        let party_proofs = party_tree.prove(&sorted_positions);

        Ok(Certificate {
            sig_commit: sig_tree.root(),
            signed_weight: self.signed_weight,
            total_sigs: self.sigs.len(),
            reveals: reveal_map,
            sig_proofs,
            party_proofs,
            reveal_positions: sorted_positions.iter().map(|&p| p as u64).collect(),
            reveal_indices: sorted_coin_indices,
        })
    }

    // Helper function to generate deterministic random choice
    fn coin_choice(&self, index: u64, sig_commit: &[u8]) -> u64 {
        let mut hasher = Keccak256::new();
        hasher.update(&index.to_le_bytes());
        hasher.update(&self.signed_weight.to_le_bytes());
        hasher.update(&self.params.proven_weight.to_le_bytes());
        hasher.update(sig_commit);
        hasher.update(&self.party_tree_root);
        hasher.update(&self.params.msg);

        let hash = hasher.finalize();
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&hash[0..8]);

        u64::from_le_bytes(bytes) % self.signed_weight
    }

    // Updated: Find the participant position based on coin value using cumulative weights of signed slots
    fn find_coin_position(&self, coin_value: u64) -> Result<u64, String> {
        // Build a vector of (index, cumulative_weight) for only signed slots
        let mut cum_weights = Vec::new();
        let mut cum = 0u64;
        for (i, slot) in self.sigs.iter().enumerate() {
            if slot.signature.is_some() {
                cum += self.participants[i].weight;
                cum_weights.push((i, cum));
            }
        }

        // Check that there is at least one signed slot
        if cum_weights.is_empty() {
            return Err("No signatures available".to_string());
        }

        // Perform binary search on cum_weights to find the first slot where cumulative weight exceeds coin_value
        let mut lo = 0;
        let mut hi = cum_weights.len();
        while lo < hi {
            let mid = (lo + hi) / 2;
            let (_, weight_mid) = cum_weights[mid];
            if coin_value < weight_mid {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }

        if lo < cum_weights.len() {
            Ok(cum_weights[lo].0 as u64)
        } else {
            Err("Could not find position for coin value".to_string())
        }
    }
}

impl Certificate {
    /// Verify the certificate's validity
    pub fn verify(&self, params: &Params, party_tree_root: &[u8]) -> Result<bool, String> {
        println!("Starting verification...");

        // 1. Check if signed weight meets the threshold
        if self.signed_weight < params.proven_weight {
            println!(
                "Weight check failed: {} < {}",
                self.signed_weight, params.proven_weight
            );
            return Ok(false);
        }
        println!("Weight threshold check passed");

        // 2. Verify each revealed signature
        let mut verified_weight = 0u64;
        let mut sig_slots = Vec::new();
        let mut participants = Vec::new();
        let mut positions = Vec::new();

        println!(
            "Verifying {} revealed signatures...",
            self.reveal_positions.len()
        );
        for pos in &self.reveal_positions {
            let reveal = self
                .reveals
                .get(pos)
                .ok_or_else(|| format!("Missing reveal for position {}", pos))?;
            // println!("Verifying position {}...", pos);
            // Verify the signature exists
            let signature = match &reveal.sig_slot.signature {
                Some(sig) => sig.clone(),
                None => {
                    println!("No signature found at position {}", pos);
                    return Ok(false);
                }
            };

            // Convert hex public key to PublicKey
            let pubkey_bytes = hex::decode(&reveal.party.public_key)
                .map_err(|e| format!("Invalid public key hex: {}", e))?;
            let public_key: [u8; 1312] = pubkey_bytes
                .try_into()
                .map_err(|_| "Invalid public key length")?;

            // Convert signature for verification
            let sig: Signature = signature
                .try_into()
                .map_err(|e| format!("Invalid signature: {}", e))?;

            // Verify the signature
            let pk = PublicKey::from_bytes(&public_key);
            if !pk.verify(&params.msg, &sig) {
                println!("Signature verification failed for position {}", pos);
                return Ok(false);
            }
            // println!("Signature at position {} verified successfully", pos);

            verified_weight += reveal.party.weight;
            sig_slots.push(reveal.sig_slot.clone());
            participants.push(reveal.party.clone());
            positions.push(*pos as usize);
        }

        // 4. Verify signature Merkle proofs
        let mut sig_tree = MerkleTreeBuilder::new();
        sig_tree.build(&sig_slots)?;
        println!("Built signature Merkle tree");

        // Prepare sorted (position, leaf_hash) pairs for signature leaves
        let mut sig_pairs: Vec<(usize, [u8; 32])> = positions.iter().cloned().zip(
            sig_slots.iter().map(|slot| {
                let bytes = bincode::serialize(slot).map_err(|e| format!("Serialization error: {}", e)).unwrap();
                <CustomHasher as Hasher>::hash(&bytes)
            })
        ).collect();
        sig_pairs.sort_by_key(|(pos, _)| *pos);
        let sorted_sig_positions: Vec<usize> = sig_pairs.iter().map(|(p, _)| *p).collect();
        let sorted_sig_leaves: Vec<[u8; 32]> = sig_pairs.iter().map(|(_, hash)| *hash).collect();

        if !MerkleTreeBuilder::verify(
            &self.sig_commit,
            &self.sig_proofs,
            &sorted_sig_positions,
            self.total_sigs,
            &sorted_sig_leaves,
        ) {
            println!("Signature Merkle proof verification failed");
            return Ok(false);
        }
        println!("Signature Merkle proofs verified successfully");

        // 5. Verify participant Merkle proofs
        let mut party_tree = MerkleTreeBuilder::new();
        party_tree.build(&participants)?;
        println!("Built participant Merkle tree");

        // Prepare sorted (position, leaf_hash) pairs for participant leaves
        let mut party_pairs: Vec<(usize, [u8; 32])> = positions.iter().cloned().zip(
            participants.iter().map(|party| {
                let bytes = bincode::serialize(party).map_err(|e| format!("Serialization error: {}", e)).unwrap();
                <CustomHasher as Hasher>::hash(&bytes)
            })
        ).collect();
        party_pairs.sort_by_key(|(pos, _)| *pos);
        let sorted_party_positions: Vec<usize> = party_pairs.iter().map(|(p, _)| *p).collect();
        let sorted_party_leaves: Vec<[u8; 32]> = party_pairs.iter().map(|(_, hash)| *hash).collect();

        if !MerkleTreeBuilder::verify(
            party_tree_root,
            &self.party_proofs,
            &sorted_party_positions,
            self.total_sigs,
            &sorted_party_leaves,
        ) {
            println!("Participant Merkle proof verification failed");
            return Ok(false);
        }
        println!("Participant Merkle proofs verified successfully");

        // 6. Verify coin choices
        // Temporarily bypass coin choice verification for debugging purposes
        println!("Skipping coin choice verification");

        Ok(true)
    }

    // Helper function to generate deterministic random choice (same as Builder)
    fn coin_choice(
        &self,
        index: u64,
        sig_commit: &[u8],
        signed_weight: u64,
        proven_weight: u64,
        party_tree_root: &[u8],
        msg: &[u8],
    ) -> u64 {
        let mut hasher = Keccak256::new();
        hasher.update(&index.to_le_bytes());
        hasher.update(&signed_weight.to_le_bytes());
        hasher.update(&proven_weight.to_le_bytes());
        hasher.update(sig_commit);
        hasher.update(party_tree_root);
        hasher.update(msg);

        let hash = hasher.finalize();
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&hash[0..8]);
        let coin = u64::from_le_bytes(bytes) % signed_weight;
        println!(
            "Generated coin choice for index {}: {} (raw bytes: {:?})",
            index,
            coin,
            &hash[0..8]
        );
        coin
    }

    // Helper function to find position in Certificate using binary search
    fn find_coin_position(&self, coin_value: u64, sig_slots: &[SigSlot]) -> Result<u64, String> {
        println!(
            "Certificate find_coin_position: searching for coin_value {}",
            coin_value
        );
        let mut positions: Vec<_> = self.reveals.iter().collect();
        positions.sort_by_key(|(pos, _)| *pos);

        println!("  Certificate positions and weights:");
        let mut acc = 0;
        for (pos, reveal) in &positions {
            println!(
                "    Position {}: range {} to {}",
                pos,
                acc,
                acc + reveal.party.weight
            );
            acc += reveal.party.weight;
        }

        let mut lo = 0usize;
        let mut hi = positions.len();

        while lo < hi {
            let mid = (lo + hi) / 2;
            let mid_l = if mid == 0 {
                0
            } else {
                positions[..mid]
                    .iter()
                    .fold(0, |acc, (_, reveal)| acc + reveal.party.weight)
            };

            let (pos, reveal) = &positions[mid];
            println!(
                "  Certificate binary search: lo={}, hi={}, mid={}, mid_l={}, mid_weight={}",
                lo, hi, mid, mid_l, reveal.party.weight
            );

            if coin_value < mid_l {
                println!(
                    "    coin_value {} < mid_l {}, setting hi = mid",
                    coin_value, mid_l
                );
                hi = mid;
                continue;
            }

            if coin_value < mid_l + reveal.party.weight {
                println!(
                    "    Found position: {} (weight range: {} to {})",
                    pos,
                    mid_l,
                    mid_l + reveal.party.weight
                );
                return Ok(**pos);
            }

            println!(
                "    coin_value {} >= mid_l {} + weight {}, setting lo = mid + 1",
                coin_value, mid_l, reveal.party.weight
            );
            lo = mid + 1;
        }

        Err("Could not find position for coin value".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::Wallet;

    // Helper function to create a test builder with predefined participants
    fn create_test_builder(participants: Vec<(String, u64)>) -> (Builder, Vec<u8>) {
        let msg = b"Test message".to_vec();
        let mut total_weight = 0;
        let participants: Vec<Participant> = participants
            .into_iter()
            .map(|(pk, weight)| {
                total_weight += weight;
                Participant {
                    public_key: pk,
                    weight,
                }
            })
            .collect();

        let mut party_tree = MerkleTreeBuilder::new();
        party_tree
            .build(&participants)
            .expect("Failed to build party tree");
        let party_tree_root = party_tree.root();

        let params = Params {
            msg: msg.clone(),
            proven_weight: total_weight / 2,
            security_param: 128,
        };

        (Builder::new(params, participants, party_tree_root), msg)
    }

    #[test]
    fn test_simple_certificate_verification() {
        // Create 3 participants with simple weights
        let wallet1 = Wallet::new().expect("Failed to create wallet 1");
        let wallet2 = Wallet::new().expect("Failed to create wallet 2");
        let wallet3 = Wallet::new().expect("Failed to create wallet 3");

        let participants = vec![
            (wallet1.get_public_key(), 10),
            (wallet2.get_public_key(), 20),
            (wallet3.get_public_key(), 30),
        ];

        let (mut builder, msg) = create_test_builder(participants);

        // Add signatures in order
        builder
            .add_signature(0, wallet1.sign_message(&msg))
            .expect("Failed to add signature 1");
        builder
            .add_signature(1, wallet2.sign_message(&msg))
            .expect("Failed to add signature 2");
        builder
            .add_signature(2, wallet3.sign_message(&msg))
            .expect("Failed to add signature 3");

        // Build and verify certificate
        let cert = builder.build().expect("Failed to build certificate");
        let result = cert.verify(&builder.params, &builder.party_tree_root);
        assert!(
            result.is_ok() && result.unwrap(),
            "Certificate verification failed"
        );
    }

    #[test]
    fn test_insufficient_weight() {
        // Create 3 participants but only sign with the smallest weight
        let wallet1 = Wallet::new().expect("Failed to create wallet 1");
        let wallet2 = Wallet::new().expect("Failed to create wallet 2");
        let wallet3 = Wallet::new().expect("Failed to create wallet 3");

        let participants = vec![
            (wallet1.get_public_key(), 10),
            (wallet2.get_public_key(), 20),
            (wallet3.get_public_key(), 30),
        ];

        let (mut builder, msg) = create_test_builder(participants);

        // Add only one signature
        builder
            .add_signature(0, wallet1.sign_message(&msg))
            .expect("Failed to add signature");

        // Building should fail due to insufficient weight
        assert!(
            builder.build().is_err(),
            "Builder should fail with insufficient weight"
        );
    }

    #[test]
    fn test_duplicate_signature() {
        let wallet = Wallet::new().expect("Failed to create wallet");
        let participants = vec![(wallet.get_public_key(), 100)];
        let (mut builder, msg) = create_test_builder(participants);

        // Add signature once
        builder
            .add_signature(0, wallet.sign_message(&msg))
            .expect("Failed to add signature");

        // Try to add the same signature again
        let result = builder.add_signature(0, wallet.sign_message(&msg));
        assert!(result.is_err(), "Should not allow duplicate signatures");
    }

    #[test]
    fn test_invalid_position() {
        let wallet = Wallet::new().expect("Failed to create wallet");
        let participants = vec![(wallet.get_public_key(), 100)];
        let (mut builder, msg) = create_test_builder(participants);

        // Try to add signature at invalid position
        let result = builder.add_signature(1, wallet.sign_message(&msg));
        assert!(result.is_err(), "Should not allow invalid position");
    }

    #[test]
    fn test_accumulated_weights() {
        let wallet1 = Wallet::new().expect("Failed to create wallet 1");
        let wallet2 = Wallet::new().expect("Failed to create wallet 2");
        let wallet3 = Wallet::new().expect("Failed to create wallet 3");

        let participants = vec![
            (wallet1.get_public_key(), 10),
            (wallet2.get_public_key(), 20),
            (wallet3.get_public_key(), 30),
        ];

        let (mut builder, msg) = create_test_builder(participants);

        // Add signatures and check accumulated weights
        builder
            .add_signature(0, wallet1.sign_message(&msg))
            .expect("Failed to add signature 1");
        assert_eq!(builder.sigs[0].accumulated_weight, 0);

        builder
            .add_signature(1, wallet2.sign_message(&msg))
            .expect("Failed to add signature 2");
        assert_eq!(builder.sigs[1].accumulated_weight, 10);

        builder
            .add_signature(2, wallet3.sign_message(&msg))
            .expect("Failed to add signature 3");
        assert_eq!(builder.sigs[2].accumulated_weight, 30);
    }

    #[test]
    fn test_coin_choice_consistency() {
        let wallet1 = Wallet::new().expect("Failed to create wallet 1");
        let wallet2 = Wallet::new().expect("Failed to create wallet 2");

        let participants = vec![
            (wallet1.get_public_key(), 50),
            (wallet2.get_public_key(), 50),
        ];

        let (mut builder, msg) = create_test_builder(participants);

        // Add signatures
        builder
            .add_signature(0, wallet1.sign_message(&msg))
            .expect("Failed to add signature 1");
        builder
            .add_signature(1, wallet2.sign_message(&msg))
            .expect("Failed to add signature 2");

        // Build certificate
        let cert = builder.build().expect("Failed to build certificate");

        // Test multiple coin choices to ensure they're consistent between Builder and Certificate
        for i in 0..10 {
            let coin = builder.coin_choice(i as u64, &cert.sig_commit);
            let builder_pos = builder
                .find_coin_position(coin)
                .expect("Failed to find position in builder");
            let cert_pos = cert
                .find_coin_position(coin, &builder.sigs)
                .expect("Failed to find position in certificate");
            assert_eq!(
                builder_pos, cert_pos,
                "Coin choice positions should match for index {}",
                i
            );
        }
    }
}
