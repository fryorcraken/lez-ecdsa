use k256::ecdsa::SigningKey;
use lez_ecdsa::VerifyInput;
use lez_ecdsa_methods::LEZ_ECDSA_ELF;
use risc0_zkvm::{ExecutorEnv, default_prover};
use serde::Deserialize;
use tiny_keccak::{Hasher, Keccak};

#[derive(Deserialize)]
struct VerifyOutput {
    recovered: [u8; 20],
    matches: bool,
}

fn keccak256(bytes: &[u8]) -> [u8; 32] {
    let mut h = Keccak::v256();
    let mut out = [0u8; 32];
    h.update(bytes);
    h.finalize(&mut out);
    out
}

fn make_test_vector() -> VerifyInput {
    let sk_bytes: [u8; 32] = [
        0x4c, 0x0a, 0xc8, 0x6f, 0x12, 0x4d, 0xa0, 0x91, 0xc7, 0x3e, 0xb8, 0x55, 0x29, 0x6e, 0xfb,
        0x10, 0x00, 0xc4, 0x4d, 0x68, 0xa9, 0xa3, 0x6e, 0x2d, 0x83, 0xb1, 0x55, 0x77, 0x91, 0x6e,
        0xab, 0xcd,
    ];
    let sk = SigningKey::from_bytes((&sk_bytes).into()).expect("seed -> sk");
    let vk = sk.verifying_key();

    let pk_uncompressed = vk.to_encoded_point(false);
    let pk_hash = keccak256(&pk_uncompressed.as_bytes()[1..]);
    let mut expected_signer = [0u8; 20];
    expected_signer.copy_from_slice(&pk_hash[12..]);

    let message = b"hello redstone".to_vec();
    let digest = keccak256(&message);

    let (sig, recovery_id) = sk
        .sign_prehash_recoverable(&digest)
        .expect("sign prehash");
    let mut signature = vec![0u8; 65];
    signature[..64].copy_from_slice(&sig.to_bytes());
    signature[64] = recovery_id.to_byte() + 27;

    VerifyInput {
        message,
        signature,
        expected_signer,
    }
}

fn main() {
    if std::env::var("RISC0_DEV_MODE").as_deref() == Ok("1") {
        eprintln!("WARN: RISC0_DEV_MODE=1 — proof is faked, numbers are NOT real measurements.");
        std::process::exit(1);
    }

    let input = make_test_vector();
    println!("expected_signer = 0x{}", hex::encode(input.expected_signer));

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
    println!("matches      = {}", out.matches);
    println!("recovered    = 0x{}", hex::encode(out.recovered));
    println!("total_cycles = {}", prove_info.stats.total_cycles);
    println!("user_cycles  = {}", prove_info.stats.user_cycles);
    println!("prove_time   = {:?}", elapsed);
    println!("receipt_size = {} bytes", receipt_bytes.len());

    if !out.matches {
        eprintln!("ERROR: recovered address did not match expected_signer");
        std::process::exit(2);
    }
}
