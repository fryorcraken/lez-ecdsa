use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
use risc0_zkvm::guest::env;
use serde::{Deserialize, Serialize};
use tiny_keccak::{Hasher, Keccak};

#[derive(Serialize, Deserialize)]
struct VerifyInput {
    message: Vec<u8>,
    signature: Vec<u8>,
    expected_signer: [u8; 20],
}

#[derive(Serialize, Deserialize)]
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

fn main() {
    let input: VerifyInput = env::read();

    let digest = keccak256(&input.message);

    let sig = Signature::from_slice(&input.signature[..64]).expect("malformed signature r||s");
    let v_norm = input.signature[64]
        .checked_sub(27)
        .expect("v must be 27 or 28");
    let recovery_id = RecoveryId::try_from(v_norm).expect("invalid recovery id");
    let pk =
        VerifyingKey::recover_from_prehash(&digest, &sig, recovery_id).expect("ecrecover failed");

    let pk_uncompressed = pk.to_encoded_point(false);
    let pk_hash = keccak256(&pk_uncompressed.as_bytes()[1..]);
    let mut recovered = [0u8; 20];
    recovered.copy_from_slice(&pk_hash[12..]);

    let out = VerifyOutput {
        matches: recovered == input.expected_signer,
        recovered,
    };
    env::commit(&out);
}
