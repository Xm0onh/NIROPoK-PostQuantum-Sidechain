use crystals_dilithium::dilithium2::{Keypair, Signature};
use serde::{Deserialize, Serialize, Deserializer, Serializer};

pub struct Wallet {
    pub keypair: Keypair,
}

impl<'de> Deserialize<'de> for Wallet {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize the keypair bytes first
        let bytes: Vec<u8> = Vec::deserialize(deserializer)?;
        
        // Convert bytes back to Keypair
        Ok(Wallet {
            keypair: Keypair::from_bytes(&bytes)
        })
    }
}

impl Serialize for Wallet {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Convert Keypair to bytes and serialize those
        self.keypair.to_bytes().serialize(serializer)
    }
}
// Implement Debug trait for Wallet
impl std::fmt::Debug for Wallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Wallet {{ keypair: <keypair> }}")
    }
}

// Implement Clone trait for Wallet
impl Clone for Wallet {
    fn clone(&self) -> Self {
        Self {
            // There is a risk of security here!
            // Why do we need to clone the keypair?
            keypair: Keypair::from_bytes(&self.keypair.to_bytes())
        }
    }
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

