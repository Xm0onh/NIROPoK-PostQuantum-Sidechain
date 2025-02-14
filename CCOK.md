
## Overview

The goal of this algorithm is to allow a set of participants—each holding a public/private key pair and an associated weight—to collaboratively sign a message. Once the total weight of the collected signatures reaches a required threshold, a certificate is built. The certificate commits to both the collected signatures and participant information using Merkle trees. Only a subset of the signatures (and the associated participant data) is "revealed" for verification in order to reduce communication overhead. Verification involves several steps, including re‑computing these coin‑flips and checking Merkle proofs.

## Components

### 1. Data Structures

- **SerializableSignature**  
  A wrapper for the Dilithium signature that provides serialization. It converts the signature into a vector of bytes and performs a length check when converting back.

- **Participant**  
  Represents a participant in the system. Each participant has a public key (in hex format) and an associated weight. The weight is used to determine the influence of each participant in reaching the threshold.

- **SigSlot**  
  Represents a slot for storing signature information. Each slot can hold an optional signature and an accumulated weight (similar to an L‑value) calculated based on the weights of preceding participants.

- **Params**  
  Contains configuration parameters for the certificate:
  - `msg`: The message that is being signed.
  - `proven_weight`: The minimum total weight (threshold) required for the certificate to be valid.
  - `security_param`: A parameter that determines how many coin flips (and hence how many reveals) will be used. A higher security parameter normally implies more reveals.

- **Reveal**  
  A reveal holds the signature slot (with the actual signature) and the associated participant information for a given revealed index.

- **Certificate**  
  The final certificate contains:
  - `sig_commit`: The root hash of the Merkle tree built from the signature slots.
  - `signed_weight`: The total weight accumulated from the collected signatures.
  - `reveals`: A mapping from a position in the tree to its corresponding reveal (signature slot and participant).
  - `sig_proofs` & `party_proofs`: Merkle proofs for the signatures and the participant data, respectively.
  - `reveal_positions`: The positions in the underlying trees (ordered as revealed by the coin flips).
  - `reveal_indices`: The original coin flip indices used to choose those reveal positions (preserved for deterministic verification).

## The Certificate Building Process (Builder)

The **Builder** is responsible for collecting signatures, building the necessary Merkle trees, and determining which indices to reveal:

1. **Signature Collection & Accumulated Weights**  
   Each participant signs the designated message. As signatures are added via `add_signature`, the builder also updates an accumulated weight for each signature slot. This accumulated weight is used later to map a "coin flip" value to a participant position.

2. **Merkle Tree Construction**  
   Two separate Merkle trees are built:
   - One from the signature slots.
   - One from the participant data.
  
   A custom hasher (using Keccak256) generates the leaf hashes for both trees. The commitment (root hash) from the signature tree is later used as an input to the coin-flip.

3. **Coin-Flipping and Reveal Determination**  
   The builder performs a number of coin flips determined by the security parameter. The coin-flip for a given index is computed using a deterministic hash function in `coin_choice`, which incorporates:
   - The coin index.
   - The total signed weight.
   - The proven (threshold) weight.
   - The signature Merkle commitment.
   - The participant Merkle commitment.
   - The message.
  
   The resulting coin value is used in a binary search (via `find_coin_position`) over the cumulative weights of the participants to select a reveal position. The builder records both the position and the coin index (stored in `reveal_positions` and `reveal_indices`) to preserve the order.

4. **Generating Proofs**  
   Based on the revealed positions, the builder calls `prove` on both Merkle tree builders to generate the corresponding proofs. These proofs provide the branch hashes necessary for a verifier to recompute the Merkle root from the revealed leaves.

5. **Certificate Assembly**  
   Finally, the certificate is assembled with the computed commitments, collected reveals, Merkle proofs, and the reveal ordering data.

## Certificate Verification

The verification process (implemented in `Certificate::verify`) involves multiple steps:

1. **Weight Verification**  
   The verifier checks that the total signed weight meets the proven threshold.

2. **Signature Verification**  
   For every revealed slot, the verifier:
   - Retrieves the participant's public key.
   - Reconstructs the signature.
   - Verifies the signature on the message using the public key.
  
   The verified weight from the reveals must still meet the threshold.

3. **Merkle Proof Verification**  
   - The verifier rebuilds the Merkle tree from the revealed signature slots and checks that the provided signature proofs match the originally committed signature root.
   - Similarly, the verifier rebuilds the participant Merkle tree and verifies the corresponding branch proofs.

4. **Coin Choice Verification**  
   Using the stored coin indices (`reveal_indices`) and positions (`reveal_positions`), the verifier recomputes the coin choices. It then verifies (via binary search) that the expected positions derived from these coin choices match the revealed positions. This step ensures that the coin flip process was not tampered with and that the revealed slots are indeed chosen honestly.

5. **Proof Size Utility**  
   An additional function, `proof_size()`, is provided in the Certificate implementation. This function calculates the total byte-size of both the signature and participant proofs – useful for performance metrics and communication cost analysis.

## Testing

The test suite includes various tests to ensure correctness:
- **test_simple_certificate_verification**: Verifies that a basic certificate (with sufficient weight) can be built and passes all verification steps.
- **test_insufficient_weight**: Ensures that certificate building fails if the total signed weight does not reach the threshold.
- **test_duplicate_signature**: Checks that a participant cannot add more than one signature.
- **test_invalid_position**: Confirms that invalid participant positions are correctly handled.
- **test_accumulated_weights** and **test_coin_choice_consistency**: Validate the proper cumulative weight updating and consistency of coin flip determinations between the Builder and the Certificate.

## Example Usage

To run the algorithm in action, you may compile and run the sample file (`src/bin/test_ccok.rs`) which:
- Generates participants with random weights.
- Simulates each participant signing a test message.
- Builds a certificate once the weight threshold is reached.
- Verifies the certificate.
- Outputs the size (in bytes) of the generated Merkle proofs.

For instance, you might see output like:
```
Certificate built successfully. Signed weight: 452
Certificate verified successfully!
Signature proof size: 512 bytes
Party proof size: 480 bytes
```

*Note:* If all leaves are revealed, the proofs may be empty (summing to 0 bytes) because there is no extra branch information needed to generate the Merkle root.

## Conclusion

This algorithm provides a compact, verifiable way to represent threshold signatures through the use of weight-based signature aggregation, Merkle tree commitments, and deterministic coin‑based reveal selection. The design ensures both efficiency and security, making it well-suited for post‑quantum and blockchain applications.

# Run a node
```
RUST_LOG=info cargo run
```

## Technical Details

### Deterministic Coin-Flipping

The `coin_choice` function computes a coin value in a deterministic yet unpredictable manner. It combines multiple inputs:

- **Coin Index:** The specific flip number (as a u64).
- **Total Signed Weight:** The sum of all weights for participants who have provided signatures.
- **Proven Weight:** The threshold weight required to build a valid certificate.
- **Signature Merkle Commitment:** The root hash of the Merkle tree built from the signature slots.
- **Participant Merkle Commitment:** The root hash of the Merkle tree built from participant data.
- **Message:** The message being signed.

These inputs are concatenated and hashed with Keccak256. The resulting hash is used to produce a coin value in the range [0, signed_weight). This design ties the randomness directly to the certificate contents, ensuring that no individual component can be manipulated independently.

### Binary Search for Position Selection

The function `find_coin_position` implements a binary search over the cumulative weight ranges of the participants:

- Each participant is assigned a weight range based on their weight. For example, the first participant covers the range [0, weight_0), the second covers [weight_0, weight_0 + weight_1), and so on.
- The coin value, computed from the coin flip, is then compared to these ranges using binary search. The function efficiently locates the participant whose weight range encompasses the coin value.
- This method ensures that participants with higher weights have a proportionally greater chance of being selected for reveal.

### Merkle Tree Commitment and Proof Generation

Both signature slots and participant data are committed using Merkle trees. Key steps include:

- **Leaf Generation:** Each element (signature slot or participant) is serialized (using bincode) and then hashed with a custom hasher based on Keccak256.
- **Tree Construction:** The rs_merkle library builds the Merkle tree from these leaves, producing a root hash that acts as a cryptographic commitment.
- **Proof Generation:** For a given set of reveal positions, proofs are generated that consist of the necessary branch hashes which a verifier can use to reconstruct the root from the revealed leaf.

This mechanism guarantees that once the roots are published, any modification of the leaves (signatures or participant data) would invalidate the proofs.

### Certificate Verification Process

The verification of a certificate involves several steps:

1. **Weight Verification:** Confirm that the sum of the weights for all revealed slots meets or exceeds the proven threshold.
2. **Signature Verification:** Each revealed signature is verified against the corresponding participant's public key and the signed message.
3. **Merkle Proof Verification:** The verifier reconstructs the Merkle trees from the revealed data using the provided proofs to check that they match the originally committed root hashes.
4. **Coin Choice Verification:** Using the stored coin indices and reveal positions, the verifier re-computes the coin choices and uses the binary search process to ensure the selected positions are correct.

### Security Considerations

- **Binding of Commitments:** Merkle trees are used to securely bind the signatures and participant data. Any tampering will result in a mismatch of the computed root hash with the original commitment.
- **Deterministic Randomness:** The coin choice is derived from multiple components using Keccak256, ensuring the process is both deterministic for verification and unpredictable for an adversary.
- **Weighted Influence:** The binary search over cumulative weights means that participants with higher weight have higher reveal probability, aligning with their influence in the threshold mechanism.

### Comparison with Go Implementation

While both the Rust and Go implementations follow the same conceptual steps, a few nuances include:
- The Rust version explicitly stores the coin flip indices (`reveal_indices`) along with the revealed positions to preserve the order, which is crucial for deterministic verification.
- Debug and error logging may differ, but the core processes of signature aggregation, Merkle tree commitment, and coin-based reveal selection are consistent across both implementations.

These technical details reinforce the design choices that ensure the efficiency, security, and integrity of the threshold certificate mechanism in a post-quantum context.
