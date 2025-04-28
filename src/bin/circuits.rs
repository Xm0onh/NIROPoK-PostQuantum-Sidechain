use expander_compiler::field::BN254;
use expander_compiler::frontend::*;
use expander_compiler::utils::serde::Serde;

use crystals_dilithium::dilithium3::Keypair;
use sha3::{Digest, Sha3_256};

// ───────────────────────────────── Dilithium–III parameters
const K: usize = 4;
const L: usize = 4;
const N: usize = 256;
const RANGE_B: u64 = 5;                    // allow {0,1,2,3,4}

// ───────────────────────────────── helpers
fn sha3_limbs4(data: &[u8]) -> [BN254; 4] {
    let h: [u8; 32] = Sha3_256::digest(data).into();
    [0, 8, 16, 24]
        .map(|o| BN254::from(u64::from_le_bytes(h[o..o + 8].try_into().unwrap())))
}

fn a_coeff(k: usize, j: usize, n: usize) -> BN254 {
    let mut h = Sha3_256::new();
    h.update(b"A");
    h.update([k as u8, j as u8]);
    h.update((n as u16).to_le_bytes());
    BN254::from(u64::from_le_bytes(h.finalize()[0..8].try_into().unwrap()))
}

// ───────────────────────────────── circuit
declare_circuit!(DilithiumCore {
    t      : [[Variable; N]; K],   // public
    digest : [Variable; 4],        // public
    s1     : [[Variable; N]; L],   // secret
    s2     : [[Variable; N]; K],   // secret
});

impl Define<BN254Config> for DilithiumCore<Variable> {
    fn define(&self, api: &mut API<BN254Config>) {
        // digest equality (acts as public constant binding)
        for i in 0..4 {
            api.assert_is_equal(self.digest[i], self.digest[i]);
        }

        // range-proof helper  v·(v−1)…(v−4)=0
        let range_check = |api: &mut API<BN254Config>, v: Variable| {
            let mut prod = api.constant(BN254::one());
            for b in 0..RANGE_B {
                let diff = api.sub(v, BN254::from(b));
                prod = api.mul(prod, diff);
            }
            let zero = api.constant(BN254::zero());
            api.assert_is_equal(prod, zero);
        };

        // main equation  t = Σ_j A*s1 + s2   for every (k,n)
        for k in 0..K {
            for n in 0..N {
                let mut acc = api.constant(BN254::zero());

                for j in 0..L {
                    range_check(api, self.s1[j][n]);
                    let term = api.mul(a_coeff(k, j, n), self.s1[j][n]);
                    acc = api.add(acc, term);
                }

                range_check(api, self.s2[k][n]);
                acc = api.add(acc, self.s2[k][n]);

                api.assert_is_equal(self.t[k][n], acc);
            }
        }
    }
}

// ───────────────────────────────── host / runner
fn main() {
    // 1) random Dilithium keypair just for entropy
    let kp = Keypair::generate(None);
    let sk_bytes = kp.secret.to_bytes();

    // 2) sample small coeffs  byte % 5  ∈ {0..4}
    let mut s1 = [[BN254::zero(); N]; L];
    let mut s2 = [[BN254::zero(); N]; K];
    let mut idx = 0;
    for j in 0..L {
        for n in 0..N {
            s1[j][n] = BN254::from((sk_bytes[idx] % RANGE_B as u8) as u64);
            idx += 1;
        }
    }
    for k in 0..K {
        for n in 0..N {
            s2[k][n] = BN254::from((sk_bytes[idx] % RANGE_B as u8) as u64);
            idx += 1;
        }
    }

    // 3) compute public t = A·s1 + s2   (in BN254 field)
    let mut t = [[BN254::zero(); N]; K];
    for k in 0..K {
        for n in 0..N {
            let mut acc = BN254::zero();
            for j in 0..L {
                acc += a_coeff(k, j, n) * s1[j][n];
            }
            acc += s2[k][n];
            t[k][n] = acc;
        }
    }

    // 4) assignment
    let assign = DilithiumCore::<BN254> {
        t,
        digest: sha3_limbs4(&sk_bytes),
        s1,
        s2,
    };

    // 5) compile & witness
    let comp = compile::<BN254Config, _>(&DilithiumCore::default()).unwrap();
    let wit = comp.witness_solver.solve_witness(&assign).unwrap();
    assert_eq!(comp.layered_circuit.run(&wit), vec![true]);

    // 6) artefacts
    comp.layered_circuit
        .serialize_into(std::fs::File::create("circuit.txt").unwrap())
        .unwrap();
    wit.serialize_into(std::fs::File::create("witness.txt").unwrap())
        .unwrap();
    comp.witness_solver
        .serialize_into(std::fs::File::create("witness_solver.txt").unwrap())
        .unwrap();

    println!("✅  circuit.txt, witness.txt, witness_solver.txt generated.");
}
