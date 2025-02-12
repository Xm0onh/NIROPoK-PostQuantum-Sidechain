use k256::{
    ecdsa::{SigningKey, VerifyingKey},
    elliptic_curve::{
        sec1::ToEncodedPoint,
        PrimeField,
    },
    ProjectivePoint, Scalar,
};
use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha512};
use std::io::{self, Read};
use aes::Aes256;
use aes::cipher::KeyIvInit;
use ctr::Ctr64BE;

// Constants matching the Go code
const SIZE_FR: usize = 32;  // secp256k1 scalar size
const SIZE_FP: usize = 32;  // secp256k1 field element size
const SIZE_PUBLIC_KEY: usize = 2 * SIZE_FP;
const SIZE_PRIVATE_KEY: usize = SIZE_FR + SIZE_PUBLIC_KEY;
const SIZE_SIGNATURE: usize = 2 * SIZE_FR;
const AES_IV: &[u8; 16] = b"gnark-crypto IV.";

// ZeroReader implementation
struct ZeroReader;

impl Read for ZeroReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        for b in buf.iter_mut() {
            *b = 0;
        }
        Ok(buf.len())
    }
}

#[derive(Debug, Clone)]
pub struct SchnorrPublicKey {
    pub key: VerifyingKey,
}

#[derive(Debug)]
pub struct SchnorrSigner {
    pub public: SchnorrPublicKey,
    scalar: [u8; SIZE_FR],
}

#[derive(Debug, Clone)]
pub struct SchnorrSignature {
    s: [u8; SIZE_FR],
    e: [u8; SIZE_FR],
}

impl SchnorrSignature {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut res = Vec::with_capacity(SIZE_SIGNATURE);
        res.extend_from_slice(&self.s);
        res.extend_from_slice(&self.e);
        res
    }

    pub fn from_bytes(buf: &[u8]) -> Result<Self, io::Error> {
        if buf.len() < SIZE_SIGNATURE {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "buffer too short"));
        }
        
        let mut sig = SchnorrSignature {
            s: [0u8; SIZE_FR],
            e: [0u8; SIZE_FR],
        };
        
        sig.s.copy_from_slice(&buf[..SIZE_FR]);
        sig.e.copy_from_slice(&buf[SIZE_FR..SIZE_SIGNATURE]);
        
        Ok(sig)
    }
}

impl SchnorrSigner {
    pub fn generate() -> Result<Self, Box<dyn std::error::Error>> {
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        
        let mut signer = SchnorrSigner {
            public: SchnorrPublicKey { key: verifying_key.clone() },
            scalar: [0u8; SIZE_FR],
        };
        
        signer.scalar.copy_from_slice(&signing_key.to_bytes());
        
        Ok(signer)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut res = Vec::with_capacity(SIZE_PRIVATE_KEY);
        res.extend_from_slice(&self.public.key.to_sec1_bytes());
        res.extend_from_slice(&self.scalar);
        res
    }

    pub fn from_bytes(buf: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        if buf.len() < SIZE_PRIVATE_KEY {
            return Err("buffer too short".into());
        }

        let verifying_key = VerifyingKey::from_sec1_bytes(&buf[..SIZE_PUBLIC_KEY])?;
        let mut scalar = [0u8; SIZE_FR];
        scalar.copy_from_slice(&buf[SIZE_PUBLIC_KEY..SIZE_PRIVATE_KEY]);

        Ok(SchnorrSigner {
            public: SchnorrPublicKey { key: verifying_key },
            scalar,
        })
    }

    pub fn sign(&self, msg: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Generate nonce
        let mut hasher = Sha512::new();
        hasher.update(&self.scalar);
        
        let mut entropy = [0u8; 32];
        OsRng.fill_bytes(&mut entropy);
        hasher.update(&entropy);
        hasher.update(msg);
        
        let key = &hasher.finalize()[..32];
        
        // Create AES-CTR CSPRNG
        type Aes256Ctr64BE = Ctr64BE<Aes256>;
        let _cipher = Aes256Ctr64BE::new(key.into(), AES_IV.into());
        
        // Generate random k
        let k = Scalar::generate_vartime(&mut OsRng);
        
        // Use GENERATOR constant
        let r = (ProjectivePoint::GENERATOR * &k).to_affine();
        
        // Use ToEncodedPoint trait
        let r_encoded = r.to_encoded_point(false);
        hasher.update(r_encoded.as_bytes());
        hasher.update(msg);
        let e = hasher.finalize();
        
        // Use from_repr_vartime for scalar conversion
        let x = Scalar::from_repr_vartime(self.scalar.into())
            .ok_or("Invalid scalar bytes")?;
        let e_scalar = Scalar::from_repr_vartime(e[..32].try_into()?)
            .ok_or("Invalid hash bytes")?;
        
        let s = k - (x * e_scalar);
        
        let mut sig = SchnorrSignature {
            s: [0u8; SIZE_FR],
            e: [0u8; SIZE_FR],
        };
        sig.s.copy_from_slice(&s.to_repr());
        sig.e.copy_from_slice(&e[..SIZE_FR]);
        
        Ok(sig.to_bytes())
    }
}

impl SchnorrPublicKey {
    pub fn verify(&self, sig: &[u8], msg: &[u8]) -> Result<bool, Box<dyn std::error::Error>> {
        let sig = SchnorrSignature::from_bytes(sig)?;
        
        let s = Scalar::from_repr_vartime(sig.s.into())
            .ok_or("Invalid signature s bytes")?;
        let e = Scalar::from_repr_vartime(sig.e.into())
            .ok_or("Invalid signature e bytes")?;
        
        let r = (ProjectivePoint::GENERATOR * &s + 
                ProjectivePoint::from(self.key.as_affine()) * &e).to_affine();
        
        let r_encoded = r.to_encoded_point(false);
        let mut hasher = Sha512::new();
        hasher.update(r_encoded.as_bytes());
        hasher.update(msg);
        let e_computed = hasher.finalize();
        
        Ok(sig.e[..] == e_computed[..SIZE_FR])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_verify() {
        let signer = SchnorrSigner::generate().unwrap();
        let msg = b"test message";
        
        // Sign
        let signature = signer.sign(msg).unwrap();
        
        // Verify
        let result = signer.public.verify(&signature, msg).unwrap();
        assert!(result, "Signature verification failed");
    }

    #[test]
    fn test_serialization() {
        let signer = SchnorrSigner::generate().unwrap();
        
        // Test signer serialization
        let bytes = signer.to_bytes();
        let recovered = SchnorrSigner::from_bytes(&bytes).unwrap();
        
        assert_eq!(signer.scalar[..], recovered.scalar[..]);
        
        // Test signature serialization
        let msg = b"test message";
        let signature = signer.sign(msg).unwrap();
        let sig = SchnorrSignature::from_bytes(&signature).unwrap();
        assert_eq!(signature, sig.to_bytes());
    }

    #[test]
    fn test_invalid_signature() {
        let signer = SchnorrSigner::generate().unwrap();
        let msg = b"test message";
        let wrong_msg = b"wrong message";
        
        let signature = signer.sign(msg).unwrap();
        
        // Verify with wrong message
        let result = signer.public.verify(&signature, wrong_msg).unwrap();
        assert!(!result, "Signature verification should fail with wrong message");
    }
} 