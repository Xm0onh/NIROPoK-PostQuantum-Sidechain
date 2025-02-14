use crystals_dilithium::dilithium2::{Keypair, Signature};
use rand::Rng;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

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
            keypair: Keypair::from_bytes(&bytes),
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

#[allow(dead_code)]
impl Wallet {
    pub fn new() -> Result<Self, String> {
        let seed = rand::thread_rng().gen::<[u8; 32]>();
        let keypair = Keypair::generate(Some(&seed));
        Ok(Self { keypair })
    }

    pub fn sign_message(&self, msg: &[u8]) -> Signature {
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
