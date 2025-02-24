use niropok_pq_sidechain::{
    ccok::{Builder, Params, Participant},
    merkle::MerkleTreeBuilder,
    wallet::Wallet,
};
use rand::Rng;

fn main() {
    // Initialize random number generator
    let mut rng = rand::thread_rng();

    // Number of participants (increased to simulate paper's scale)
    let num_participants = 5000;
    let mut total_weight: u64 = 0;

    // Create participants and wallets
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
    }
    println!("Participants created");
    // Define message to sign
    let msg = b"Threshold signature test message".to_vec();

    // We'll simulate a scenario where only 80% of totalWeight is signed
    let target_signed_weight = (total_weight as f64 * 0.8).round() as u64;
    println!(
        "Total weight: {}. Target signed weight (80%% of total): {}",
        total_weight, target_signed_weight
    );

    // Define provenWeight percentages as fractions of totalWeight (10%, 30%, 50%, 70%)
    let proven_ratios = vec![0.10, 0.30, 0.50, 0.70];

    // Use a high constant security parameter to allow for many coin flips
    let security_param = 1000;

    // Iterate over different provenWeight percentages
    for ratio in proven_ratios {
        let proven_weight = (total_weight as f64 * ratio).round() as u64;
        println!(
            "\n===== Experiment with provenWeight = {}%% of total (i.e., {}) =====",
            ratio * 100.0,
            proven_weight
        );

        // Build party tree from participants
        let mut party_tree = MerkleTreeBuilder::new();
        party_tree
            .build(&participants)
            .expect("Failed to build party tree");
        let party_tree_root = party_tree.root();

        // Create parameters with the current provenWeight and constant security parameter
        let params = Params {
            msg: msg.clone(),
            proven_weight,
            security_param,
        };

        // Create the Builder
        let mut builder = Builder::new(params, participants.clone(), party_tree_root.clone());

        // Each participant signs the message until we reach the target signed weight (80% of totalWeight)
        println!(
            "Collecting signatures until cumulative weight reaches {}...",
            target_signed_weight
        );
        let mut signed_count = 0;
        for (i, wallet) in wallets.iter().enumerate() {
            if builder.signed_weight < target_signed_weight {
                let signature = wallet.sign_message(&msg);
                if builder.add_signature(i, signature).is_ok() {
                    signed_count += 1;
                }
            }
        }
        println!(
            "Collected signatures from {} participants; final cumulative signed weight: {}",
            signed_count, builder.signed_weight
        );

        // Build the certificate
        println!(
            "Building certificate for provenWeight {} ({}%% of total)...",
            proven_weight,
            ratio * 100.0
        );
        let cert = match builder.build() {
            Ok(cert) => cert,
            Err(e) => {
                println!("Error building certificate: {}", e);
                continue;
            }
        };
        println!(
            "Certificate built successfully. Signed weight: {}",
            cert.signed_weight
        );

        // Verify the certificate
        println!("Verifying certificate...");
        match cert.verify(&builder.params, &party_tree_root) {
            Ok(true) => println!("Certificate verified successfully!"),
            Ok(false) => println!("Certificate verification failed!"),
            Err(e) => println!("Error during verification: {}", e),
        }

        // Output the number of reveals and proof sizes
        println!(
            "Number of reveals in certificate: {}",
            cert.reveal_positions.len()
        );
        let (sig_size, party_size) = cert.proof_size();
        println!("Signature proof size: {} bytes", sig_size);
        println!("Party proof size: {} bytes", party_size);
    }
}
