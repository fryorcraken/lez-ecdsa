use k256::ecdsa::SigningKey;
use lez_ecdsa::{SignerVerification, VerifyInput, VerifyOutput};
use lez_ecdsa_methods::LEZ_ECDSA_ELF;
use risc0_zkvm::{ExecutorEnv, default_prover};
use tiny_keccak::{Hasher, Keccak};

const SEEDS: &[[u8; 32]] = &[
    [
        0x4c, 0x0a, 0xc8, 0x6f, 0x12, 0x4d, 0xa0, 0x91, 0xc7, 0x3e, 0xb8, 0x55, 0x29, 0x6e, 0xfb,
        0x10, 0x00, 0xc4, 0x4d, 0x68, 0xa9, 0xa3, 0x6e, 0x2d, 0x83, 0xb1, 0x55, 0x77, 0x91, 0x6e,
        0xab, 0xcd,
    ],
    [
        0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66,
        0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66,
        0x77, 0x01,
    ],
    [
        0xa1, 0xb2, 0xc3, 0xd4, 0xe5, 0xf6, 0x07, 0x18, 0x29, 0x3a, 0x4b, 0x5c, 0x6d, 0x7e, 0x8f,
        0x90, 0xa1, 0xb2, 0xc3, 0xd4, 0xe5, 0xf6, 0x07, 0x18, 0x29, 0x3a, 0x4b, 0x5c, 0x6d, 0x7e,
        0x8f, 0x02,
    ],
];

fn keccak256(bytes: &[u8]) -> [u8; 32] {
    let mut h = Keccak::v256();
    let mut out = [0u8; 32];
    h.update(bytes);
    h.finalize(&mut out);
    out
}

fn make_test_vector(num_signers: usize) -> VerifyInput {
    assert!(num_signers <= SEEDS.len(), "only {} seeds available", SEEDS.len());

    let message = b"hello redstone".to_vec();
    let digest = keccak256(&message);

    let signers = SEEDS
        .iter()
        .take(num_signers)
        .map(|seed| {
            let sk = SigningKey::from_bytes(seed.into()).expect("seed -> sk");
            // Compressed SEC1 pubkey (33 bytes).
            let pubkey = sk.verifying_key().to_encoded_point(true).as_bytes().to_vec();

            let (sig, _recovery_id) = sk.sign_prehash_recoverable(&digest).expect("sign prehash");
            let signature = sig.to_bytes().to_vec();

            SignerVerification { pubkey, signature }
        })
        .collect();

    VerifyInput { message, signers }
}

fn main() {
    if std::env::var("RISC0_DEV_MODE").as_deref() == Ok("1") {
        eprintln!("WARN: RISC0_DEV_MODE=1 — proof is faked, numbers are NOT real measurements.");
        std::process::exit(1);
    }

    let num_signers: usize = std::env::var("LEZ_ECDSA_SIGNERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);

    println!("num_signers  = {num_signers}");
    let input = make_test_vector(num_signers);
    for (i, s) in input.signers.iter().enumerate() {
        println!("signer[{i}]    = pubkey 0x{}", hex::encode(&s.pubkey));
    }

    let env = ExecutorEnv::builder()
        .write(&input)
        .expect("write input")
        .build()
        .expect("build env");

    let prover = default_prover();

    let t0 = std::time::Instant::now();
    let prove_info = prover.prove(env, LEZ_ECDSA_ELF).expect("prove");
    let elapsed = t0.elapsed();

    let receipt = &prove_info.receipt;
    let receipt_bytes = bincode::serialize(receipt).expect("serialize receipt");
    let out: VerifyOutput = receipt.journal.decode().expect("decode journal");

    println!("---");
    println!("all_valid    = {}", out.all_valid);
    println!("valid_count  = {}", out.valid_count);
    println!("total_cycles = {}", prove_info.stats.total_cycles);
    println!("user_cycles  = {}", prove_info.stats.user_cycles);
    println!("prove_time   = {:?}", elapsed);
    println!("receipt_size = {} bytes", receipt_bytes.len());

    if !out.all_valid {
        eprintln!("ERROR: at least one signer did not verify");
        std::process::exit(2);
    }
}
