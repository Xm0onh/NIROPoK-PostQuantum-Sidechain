use rs_merkle::{Hasher, MerkleProof, MerkleTree};
use serde::Serialize;
use sha3::{Digest, Keccak256};

/// Custom hasher using Keccak256 (SHA3)
#[derive(Default, Clone)]
pub struct CustomHasher(Keccak256);

impl Hasher for CustomHasher {
    type Hash = [u8; 32];

    fn hash(data: &[u8]) -> Self::Hash {
        let mut hasher = Keccak256::new();
        hasher.update(data);
        hasher.finalize().into()
    }
}

pub struct MerkleTreeBuilder {
    tree: MerkleTree<CustomHasher>,
}

impl MerkleTreeBuilder {
    /// Create a new empty Merkle tree
    pub fn new() -> Self {
        Self {
            tree: MerkleTree::new(),
        }
    }

    /// Build a Merkle tree from a list of serializable items
    pub fn build<T: Serialize>(&mut self, items: &[T]) -> Result<(), String> {
        let leaves: Vec<[u8; 32]> = items
            .iter()
            .map(|item| {
                let bytes =
                    bincode::serialize(item).map_err(|e| format!("Serialization error: {}", e))?;
                Ok(CustomHasher::hash(&bytes))
            })
            .collect::<Result<Vec<_>, String>>()?;

        self.tree = MerkleTree::<CustomHasher>::from_leaves(&leaves);
        Ok(())
    }

    /// Get the root hash of the Merkle tree
    pub fn root(&self) -> Vec<u8> {
        self.tree.root().unwrap_or_default().to_vec()
    }

    /// Generate Merkle proofs for given positions
    pub fn prove(&self, positions: &[usize]) -> Vec<Vec<u8>> {
        let proof = self.tree.proof(positions);
        proof
            .proof_hashes()
            .iter()
            .map(|hash| hash.to_vec())
            .collect()
    }

    /// Verify a Merkle proof
    pub fn verify(
        root: &[u8],
        proof_hashes: &[Vec<u8>],
        positions: &[usize],
        total_leaves: usize,
        leaves: &[[u8; 32]],
    ) -> bool {
        let proof = MerkleProof::<CustomHasher>::new(
            proof_hashes
                .iter()
                .map(|h| {
                    let mut hash = [0u8; 32];
                    hash.copy_from_slice(h);
                    hash
                })
                .collect(),
        );

        let mut root_hash = [0u8; 32];
        root_hash.copy_from_slice(root);

        proof.verify(root_hash, positions, leaves, total_leaves)
    }
}

impl Default for MerkleTreeBuilder {
    fn default() -> Self {
        Self::new()
    }
}
