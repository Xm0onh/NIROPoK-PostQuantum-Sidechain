use niropok_pq_sidechain::{
    ccok::{Builder, Params, Participant},
    merkle::MerkleTreeBuilder,
    wallet::Wallet,
};
use rand::Rng;

fn main() {
    // Initialize random number generator
    let mut rng = rand::thread_rng();

    // Number of participants
    let num_participants = 10;
    let mut total_weight: u64 = 0;

    // Create participants with wallets and random weights
    let mut participants = Vec::with_capacity(num_participants);
    let mut wallets = Vec::with_capacity(num_participants);

    println!("Generating {} participants...", num_participants);
    for i in 0..num_participants {
        // Generate wallet (contains Dilithium keypair)
        let wallet = Wallet::new().expect("Failed to create wallet");

        // Assign random weight between 10 and 100
        let weight = rng.gen_range(10..=100);
        total_weight += weight;

        participants.push(Participant {
            public_key: wallet.get_public_key(),
            weight,
        });
        wallets.push(wallet);
        println!("Participant {} created with weight {}", i, weight);
    }

    // Define message to sign
    let msg = b"Threshold signature test message".to_vec();

    // Set proven threshold as half of total weight
    let proven_weight = total_weight / 2;
    println!(
        "Total weight: {}, Proven threshold (ProvenWeight): {}",
        total_weight, proven_weight
    );

    // Build party tree from participants
    let mut party_tree = MerkleTreeBuilder::new();
    party_tree
        .build(&participants)
        .expect("Failed to build party tree");
    let party_tree_root = party_tree.root();

    // Create parameters
    let params = Params {
        msg: msg.clone(),
        proven_weight,
        security_param: 32, // Same security parameter as Go implementation
    };

    // Create the Builder
    let mut builder = Builder::new(params, participants, party_tree_root.clone());

    // Each participant signs the message
    println!("\nCollecting signatures...");
    for (i, wallet) in wallets.iter().enumerate() {
        let signature = wallet.sign_message(&msg);

        if let Err(e) = builder.add_signature(i, signature) {
            println!("Error adding signature for participant {}: {}", i, e);
            return;
        }
        println!("Participant {} signed successfully", i);
    }

    // Build the certificate
    println!("\nBuilding certificate...");
    let cert = match builder.build() {
        Ok(cert) => cert,
        Err(e) => {
            println!("Error building certificate: {}", e);
            return;
        }
    };
    println!(
        "Certificate built successfully. Signed weight: {}",
        cert.signed_weight
    );

    // Verify the certificate
    println!("\nVerifying certificate...");
    match cert.verify(&builder.params, &party_tree_root) {
        Ok(true) => println!("Certificate verified successfully!"),
        Ok(false) => println!("Certificate verification failed!"),
        Err(e) => println!("Error during verification: {}", e),
    }

    let (sig_size, party_size) = cert.proof_size();
    println!("Signature proof size: {} bytes", sig_size);
    println!("Party proof size: {} bytes", party_size);
}
