// src/bin/pqzk_offchain_sha3.rs
use expander_compiler::frontend::*;
use expander_compiler::utils::serde::Serde;

use expander_compiler::field::BN254;
use sha3::{Digest, Sha3_512};

declare_circuit!(PQZKOffchain {
    seed_a: Variable,
    r_s: Variable,
    sk_s: Variable,   // private
    h_seed: Variable, // public (SHA3‑512 digest)
    pk_m: Variable,
    c_a: Variable,
    pk_s: Variable, // public
});

impl Define<BN254Config> for PQZKOffchain<Variable> {
    fn define(&self, b: &mut API<BN254Config>) {
        // 1) pk_m = h_seed + 1
        let pk_m_computed = b.add(self.h_seed, BN254::one());
        b.assert_is_equal(self.pk_m, pk_m_computed);

        // 2) c_a  = h_seed  + r_s   (still toy, just to keep structure)
        let c_a_computed = b.add(self.h_seed, self.r_s);
        b.assert_is_equal(self.c_a, c_a_computed);

        // 3) pk_s = sk_s   + 1
        let pk_s_computed = b.add(self.sk_s, BN254::one());
        b.assert_is_equal(self.pk_s, pk_s_computed);
    }
}

fn main() {
    // compile once
    let compile_result = compile::<BN254Config, _>(&PQZKOffchain::default()).unwrap();

    // demo numbers
    let (seed, rs, sks) = (123u64, 456u64, 789u64);

    // real SHA‑3‑512 off‑circuit
    let mut hasher = Sha3_512::new();
    hasher.update(seed.to_be_bytes());
    let digest = hasher.finalize(); // 64‑byte output
                                    // Take first 8 bytes and convert to u64, then to BN254
    let first_8_bytes: [u8; 8] = digest[0..8].try_into().unwrap();
    let h_seed = BN254::from(u64::from_be_bytes(first_8_bytes));

    let assignment = PQZKOffchain::<BN254> {
        seed_a: BN254::from(seed), // private
        r_s: BN254::from(rs),      // private
        sk_s: BN254::from(sks),    // private

        h_seed,                                // public
        pk_m: h_seed + BN254::one(),           // public
        c_a: h_seed + BN254::from(rs),         // toy commitment
        pk_s: BN254::from(sks) + BN254::one(), // public
    };

    let witness = compile_result
        .witness_solver
        .solve_witness(&assignment)
        .unwrap();
    let output = compile_result.layered_circuit.run(&witness);
    assert_eq!(output, vec![true]);

    let file = std::fs::File::create("circuit.txt").unwrap();
    let writer = std::io::BufWriter::new(file);

    compile_result
        .layered_circuit
        .serialize_into(writer)
        .unwrap();

    // Serialize and write the witness to a file
    let file = std::fs::File::create("witness.txt").unwrap();
    let writer = std::io::BufWriter::new(file);
    witness.serialize_into(writer).unwrap();

    // Serialize and write the witness solver to a file
    let file = std::fs::File::create("witness_solver.txt").unwrap();
    let writer = std::io::BufWriter::new(file);
    compile_result
        .witness_solver
        .serialize_into(writer)
        .unwrap();
}
