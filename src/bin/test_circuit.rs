use expander_compiler::frontend::*;
use internal::Serde;

// Import the SHA3 crate (we may use it externally)
use sha3::{Digest, Sha3_256};
use std::convert::TryInto;

// This function computes a SHA3-256 hash for a u64 value.
// In a real application you might use all the hash bits, but here we take the first 8 bytes.
fn compute_sha3_hash(value: u64) -> u64 {
    let mut hasher = Sha3_256::new();
    hasher.update(&value.to_be_bytes());
    let result = hasher.finalize();
    let bytes: [u8; 8] = result[0..8]
        .try_into()
        .expect("slice with incorrect length");
    u64::from_be_bytes(bytes)
}

// This function is meant to represent a SHA3 hash gadget inside the circuit.
// Since incorporating a full SHA3 gadget is complex, for now we simulate it by
// simply adding a constant (42) to the input.
// In a production circuit you would replace this placeholder with a proper SHA3 circuit.
fn sha3_hash_variable(builder: &mut API<GF2Config>, input: Variable) -> Variable {
    builder.add(input, GF2::from(42))
}

declare_circuit!(Circuit {
    x: Variable,
    y: Variable,
    target: Variable,
});

impl Define<GF2Config> for Circuit<Variable> {
    fn define(&self, builder: &mut API<GF2Config>) {
        let mut x = self.x;
        let mut y = self.y;
        for _ in 0..30 {
            let temp = builder.add(x, y);
            x = y;
            y = temp;
        }
        builder.assert_is_equal(y, self.target);
    }
}

declare_circuit!(PQZKCircuit {
    seed_a: Variable,
    r_s: Variable,
    sk_s: Variable,
    pk_m: Variable,
    c_a: Variable,
    pk_s: Variable,
});

impl Define<GF2Config> for PQZKCircuit<Variable> {
    fn define(&self, builder: &mut API<GF2Config>) {
        // Step 1: Compute hashed_seed = SHA3(SeedA) using our gadget.
        let hashed_seed = sha3_hash_variable(builder, self.seed_a);
        // Simulate key generation: add 1 to the hash.
        let computed_pk_m = builder.add(hashed_seed, GF2::from(1));
        builder.assert_is_equal(computed_pk_m, self.pk_m);

        // Step 2: Compute the commitment c_a = SHA3(SeedA || rS).
        // Here we simulate concatenation by adding seed_a and r_s first.
        let commitment_input = builder.add(self.seed_a, self.r_s);
        let computed_c_a = sha3_hash_variable(builder, commitment_input);
        builder.assert_is_equal(computed_c_a, self.c_a);

        // Step 3: Compute pk_s = PQKGen(sk_s), simulated by adding 1.
        let computed_pk_s = builder.add(self.sk_s, GF2::from(1));
        builder.assert_is_equal(computed_pk_s, self.pk_s);
    }
}

fn main() {
    let compile_result = compile(&PQZKCircuit::default()).unwrap();

    // For demonstration, use simplified GF2 values.
    // In our circuit, the SHA3 gadget is simulated as: sha3(x) = x + 42.
    // Thus for seed_a = 123:
    //   hashed_seed = 123 + 42 = 165, then pk_m = 165 + 1 = 166.
    // For the commitment: seed_a + r_s = 123 + 456 = 579, then c_a = 579 + 42 = 621.
    // And for pk_s: sk_s = 789, then pk_s = 789 + 1 = 790.
    let seed_a_val: u32 = 123;
    let r_s_val: u32 = 456;
    let sk_s_val: u32 = 789;
    let simulated_hashed_seed = seed_a_val.wrapping_add(42); // 123 + 42 = 165
    let pk_m_val = simulated_hashed_seed.wrapping_add(1); // 165 + 1 = 166
    let commitment_input_val = seed_a_val.wrapping_add(r_s_val); // 123 + 456 = 579
    let c_a_val = commitment_input_val.wrapping_add(42); // 579 + 42 = 621
    let pk_s_val = sk_s_val.wrapping_add(1); // 789 + 1 = 790

    let assignment = PQZKCircuit::<GF2> {
        seed_a: GF2::from(seed_a_val),
        r_s: GF2::from(r_s_val),
        sk_s: GF2::from(sk_s_val),
        pk_m: GF2::from(pk_m_val),
        c_a: GF2::from(c_a_val),
        pk_s: GF2::from(pk_s_val),
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
