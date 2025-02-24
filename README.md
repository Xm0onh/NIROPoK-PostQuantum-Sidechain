# NIROPoK Post-Quantum

## How to run a node

```bash
RUST_LOG=info cargo run
```

## HashChain Mechanism

This project utilizes a hash chain to ensure fairness and unpredictability in block production.

1. **Generation of Hash Chain:**  
   At the start of each epoch, a new hash chain is generated by invoking `HashChain::new()`.  
   This chain is a sequence of precomputed hash values.

2. **Commitment via Final Hash:**  
   The last hash in the chain serves as a commitment for the epoch.  
   This commitment is broadcast to the network as a `HashChainCom` message, ensuring all nodes share a common, verifiable reference.

3. **Randomness for Block Proposal:**  
   During mining events, a specific hash is selected from the chain based on the epoch's progression.  
   This hash acts as a seed, determining which node is eligible to propose the next block.  
   By relying on a precommitted hash chain, the protocol prevents tampering and manipulation.

4. **Verification Process:**  
   As epochs progress, parts of the hash chain are revealed, allowing nodes to verify that the selection process aligns with the initial commitment.  
   This mechanism ensures that the randomness and fairness in the block production process are maintained across the peer-to-peer network.