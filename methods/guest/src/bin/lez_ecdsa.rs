use k256::ecdsa::{Signature, VerifyingKey, signature::hazmat::PrehashVerifier};
use risc0_zkvm::guest::env;
use serde::{Deserialize, Serialize};
use tiny_keccak::{Hasher, Keccak};

#[derive(Serialize, Deserialize)]
struct SignerVerification {
    /// SEC1-encoded pubkey (33 bytes compressed, or 65 uncompressed).
    pubkey: Vec<u8>,
    /// 64 bytes: r || s. No recovery byte — `verify`, not `recover`.
    signature: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
struct VerifyInput {
    message: Vec<u8>,
    signers: Vec<SignerVerification>,
}

#[derive(Serialize, Deserialize)]
struct VerifyOutput {
    valid_count: u32,
    all_valid: bool,
}

fn keccak256(bytes: &[u8]) -> [u8; 32] {
    let mut h = Keccak::v256();
    let mut out = [0u8; 32];
    h.update(bytes);
    h.finalize(&mut out);
    out
}

fn verify_one(digest: &[u8; 32], pubkey: &[u8], signature: &[u8]) -> bool {
    let vk = match VerifyingKey::from_sec1_bytes(pubkey) {
        Ok(vk) => vk,
        Err(_) => return false,
    };
    let sig = match Signature::from_slice(signature) {
        Ok(s) => s,
        Err(_) => return false,
    };
    vk.verify_prehash(digest, &sig).is_ok()
}

fn main() {
    let input: VerifyInput = env::read();
    let digest = keccak256(&input.message);

    let mut valid_count: u32 = 0;
    for s in &input.signers {
        if verify_one(&digest, &s.pubkey, &s.signature) {
            valid_count += 1;
        }
    }

    env::commit(&VerifyOutput {
        all_valid: valid_count as usize == input.signers.len(),
        valid_count,
    });
}
