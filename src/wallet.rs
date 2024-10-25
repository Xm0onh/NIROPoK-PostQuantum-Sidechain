use crystals_dilithium::dilithium2::{Keypair, Signature};

// use serde::{Deserialize, Serialize};
// use rand::rngs::OsRng;

pub struct Wallet {
    pub keypair: Keypair,
}
#[allow(dead_code)]
impl Wallet {
    pub fn new() -> Result<Self, String> {
        let seed = [0u8; 32]; // Define a seed with 32 bytes
        let keypair = Keypair::generate(Some(&seed));
        Ok(Self{
            keypair,
        })
    }

    pub fn sign(&self, msg: &[u8]) -> Signature {
        self.keypair.sign(msg)
    }

    pub fn verify(&self, msg: &[u8], signature: &Signature) -> bool {
        self.keypair.public.verify(msg, signature)
    }

    pub fn get_public_key(&self) -> String {
        hex::encode(self.keypair.public.to_bytes())
    }

    pub fn get_private_key(&self) -> String {
        hex::encode(self.keypair.secret.to_bytes())
    }
}
