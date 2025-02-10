use sha3::{Sha3_256, Digest};
use crypto_common::Reset;
use sha3::digest::FixedOutputReset;
use rayon::prelude::*;
use std::collections::BTreeMap;

// Helper function equivalent to Go's HashSum.
fn hash_sum(hasher: &mut (impl Digest + Reset + FixedOutputReset), data: &[&[u8]]) -> Vec<u8> {
    for d in data {
        Digest::update(hasher, d);
    }
    hasher.finalize_reset().to_vec()
}

#[derive(Debug)]
pub struct MerkleTree {
    // Function pointer that creates a new hasher.
    pub new_hasher: fn() -> Sha3_256,
    // Each layer is a Vec of hash bytes.
    pub hashes: Vec<Vec<Vec<u8>>>,
}

impl MerkleTree {
    pub fn new() -> Self {
        MerkleTree {
            new_hasher: Sha3_256::new, // default to SHA3-256
            hashes: Vec::new(),
        }
    }

    // Similar to WithHasher in Go.
    pub fn with_hasher(mut self, f: fn() -> Sha3_256) -> Self {
        self.new_hasher = f;
        self
    }

    // Build the Merkle tree from a slice of leaf data.
    pub fn build(mut self, data: &[Vec<u8>]) -> Self {
        let num_data = data.len();
        let new_hasher = self.new_hasher;
        // Compute leaves concurrently.
        let leaves: Vec<Vec<u8>> = (0..num_data)
            .into_par_iter()
            .map(|i| {
                let mut hasher = (new_hasher)();
                hash_sum(&mut hasher, &[&data[i]])
            })
            .collect();
        self.hashes.push(leaves);

        // Build internal nodes concurrently.
        while self.hashes.last().unwrap().len() > 1 {
            let prev_layer = self.hashes.last().unwrap();
            let new_len = (prev_layer.len() + 1) / 2;
            let new_layer: Vec<Vec<u8>> = (0..new_len)
                .into_par_iter()
                .map(|k| {
                    let mut hasher = (self.new_hasher)();
                    let left = &prev_layer[2 * k];
                    let right = if 2 * k + 1 < prev_layer.len() {
                        prev_layer[2 * k + 1].as_slice()
                    } else {
                        &[]
                    };
                    hash_sum(&mut hasher, &[left, right])
                })
                .collect();
            self.hashes.push(new_layer);
        }

        self
    }

    // Returns the Merkle root (or None if the tree is empty).
    pub fn root(&self) -> Option<&Vec<u8>> {
        if self.hashes.is_empty() || self.hashes[0].is_empty() {
            None
        } else {
            self.hashes.last().unwrap().get(0)
        }
    }

    // Prove a set of leaf indexes. Returns a vector of sibling hashes (or an error).
    pub fn prove(&self, idxs: &[usize]) -> Result<Vec<Vec<u8>>, String> {
        if self.hashes.is_empty() || self.hashes[0].is_empty() {
            return Err("empty tree".to_string());
        }

        let mut proof = Vec::new();
        let mut known_idxs = idxs.to_vec();
        known_idxs.sort_unstable();
        known_idxs.dedup();

        // Validate indices.
        for &idx in &known_idxs {
            if idx >= self.hashes[0].len() {
                return Err(format!("invalid index {}", idx));
            }
        }

        let mut current_known = known_idxs;
        // For every tree level except the root.
        for level in 0..(self.hashes.len() - 1) {
            let layer = &self.hashes[level];
            let mut new_known = Vec::with_capacity((current_known.len() + 1) / 2);
            let mut i = 0;
            while i < current_known.len() {
                let idx = current_known[i];
                if idx % 2 == 0 {
                    if i + 1 < current_known.len() && current_known[i + 1] == idx + 1 {
                        // Sibling already known; skip adding proof.
                        i += 2;
                    } else {
                        if idx + 1 < layer.len() {
                            proof.push(layer[idx + 1].clone());
                        } else {
                            proof.push(vec![]);
                        }
                        i += 1;
                    }
                } else {
                    proof.push(layer[idx - 1].clone());
                    i += 1;
                }
                new_known.push(idx / 2);
            }
            current_known = new_known;
        }

        Ok(proof)
    }
}

// Internal helper for Verify functions.
#[derive(Debug)]
struct ProofItem {
    idx: usize,
    hash: Vec<u8>,
}

// VerifyMerkleTreeWithHash (and VerifyMerkleTree) are implemented below.
// Since Go uses maps (which are unsorted) we require a sorted collection;
// here we use a BTreeMap for equivalent behavior.

pub fn verify_merkle_tree(root: &[u8], elems: &BTreeMap<usize, Vec<u8>>, proof: &[Vec<u8>]) -> Result<(), String> {
    verify_merkle_tree_with_hash(root, elems, proof)
}

pub fn verify_merkle_tree_with_hash(root: &[u8], elems: &BTreeMap<usize, Vec<u8>>, proof: &[Vec<u8>]) -> Result<(), String> {
    if elems.is_empty() {
        return Ok(());
    }

    // Build the initial partial layer from elems.
    let mut partial_layer: Vec<ProofItem> = elems
        .iter()
        .map(|(&idx, hash)| ProofItem {
            idx,
            hash: hash.clone(),
        })
        .collect();
    partial_layer.sort_by_key(|p| p.idx);

    let mut p_id = 0;
    while !partial_layer.is_empty() && partial_layer[0].hash != root {
        let mut next_partial_layer = Vec::with_capacity((partial_layer.len() + 1) / 2);
        let mut i = 0;
        while i < partial_layer.len() {
            let p = &partial_layer[i];
            let (left, right) = if p.idx % 2 == 0 {
                let left = p.hash.clone();
                if i + 1 < partial_layer.len() && partial_layer[i + 1].idx == p.idx + 1 {
                    let right = partial_layer[i + 1].hash.clone();
                    i += 2;
                    (left, right)
                } else {
                    if p_id >= proof.len() {
                        return Err("invalid proof".to_string());
                    }
                    let right = proof[p_id].clone();
                    p_id += 1;
                    i += 1;
                    (left, right)
                }
            } else {
                if p_id >= proof.len() {
                    return Err("invalid proof".to_string());
                }
                let left = proof[p_id].clone();
                p_id += 1;
                let right = p.hash.clone();
                i += 1;
                (left, right)
            };
            let mut hasher = Sha3_256::new();
            let combined = hash_sum(&mut hasher, &[&left, &right]);
            next_partial_layer.push(ProofItem {
                idx: p.idx / 2,
                hash: combined,
            });
        }
        partial_layer = next_partial_layer;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_build_and_root() {
        // Build a tree with four leaves.
        let data = vec![
            b"leaf1".to_vec(),
            b"leaf2".to_vec(),
            b"leaf3".to_vec(),
            b"leaf4".to_vec(),
        ];
        let merkle_tree = MerkleTree::new().build(&data);
        let root = merkle_tree.root();
        assert!(root.is_some(), "Merkle root should exist");
    }

    #[test]
    fn test_prove_and_verify() {
        // Build a tree with four leaves.
        let data = vec![
            b"a".to_vec(),
            b"b".to_vec(),
            b"c".to_vec(),
            b"d".to_vec(),
        ];
        let merkle_tree = MerkleTree::new().build(&data);
        let root = merkle_tree.root().expect("root exists").clone();

        // Generate proof for indices 1 and 2.
        let indices = vec![1, 2];
        let proof = merkle_tree.prove(&indices).expect("proof generated");

        // Create a BTreeMap for the proven leaves (using the leaf level).
        let mut elems = BTreeMap::new();
        for &i in &indices {
            let mut hasher = (merkle_tree.new_hasher)();
            let leaf_hash = hash_sum(&mut hasher, &[&data[i]]);
            elems.insert(i, leaf_hash);
        }

        // Verify the proof; it should succeed.
        let result = verify_merkle_tree(&root, &elems, &proof);
        assert!(result.is_ok(), "Proof verification should succeed");
    }

    #[test]
    fn test_invalid_index_in_prove() {
        // With only two leaves, index 2 is invalid.
        let data = vec![
            b"test1".to_vec(),
            b"test2".to_vec(),
        ];
        let merkle_tree = MerkleTree::new().build(&data);
        let result = merkle_tree.prove(&[2]);  // Out of range index.
        assert!(result.is_err(), "Proving invalid index should fail");
    }

    #[test]
    fn test_verify_with_wrong_proof() {
        // Build a tree with three leaves.
        let data = vec![
            b"foo".to_vec(),
            b"bar".to_vec(),
            b"baz".to_vec(),
        ];
        let merkle_tree = MerkleTree::new().build(&data);
        let root = merkle_tree.root().expect("root exists").clone();

        // Prove for index 0.
        let indices = vec![0];
        let proof = merkle_tree.prove(&indices).expect("proof generated");

        // Tamper with the proof to simulate an invalid proof.
        let tampered_proof: Vec<Vec<u8>> = proof.into_iter().map(|mut p| {
            if !p.is_empty() {
                p[0] ^= 0xFF; // flip bits in first byte
            }
            p
        }).collect();

        let mut elems = BTreeMap::new();
        let mut hasher = (merkle_tree.new_hasher)();
        let leaf_hash = hash_sum(&mut hasher, &[&data[0]]);
        elems.insert(0, leaf_hash);

        let result = verify_merkle_tree(&root, &elems, &tampered_proof);
        assert!(result.is_err(), "Verification with tampered proof should fail");
    }

    #[test]
    fn test_empty_tree() {
        // Building an empty tree should yield no root.
        let data: Vec<Vec<u8>> = vec![];
        let merkle_tree = MerkleTree::new().build(&data);
        assert!(merkle_tree.root().is_none(), "Empty tree should have no root");
    }
}
