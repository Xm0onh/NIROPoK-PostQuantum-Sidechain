use crystals_dilithium::dilithium2::Keypair;
use serde::{Deserialize, Serialize};
use rand::rngs::OsRng;


pub struct Wallet {
    pub public_key: Keypair::Public,
    pub private_key: Keypair::Secret,
}


// fn main() {
//     let seed = [0u8; 32]; // Define a seed with 32 bytes
//     let msg = "Hello, world!";
//     let keypair = Keypair::generate(Some(&seed));
//     let signature = keypair.sign(&msg.as_bytes());
//     let is_verified = keypair.public.verify(&msg.as_bytes(), &signature);
//     println!("Signature: {:?}", signature);
//     println!("Is verified: {}", is_verified);
// }

