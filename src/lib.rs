use k256::ecdsa::SigningKey;
use serde::{Deserialize, Serialize};
use tiny_keccak::{Hasher, Keccak};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SignerVerification {
    /// SEC1-encoded pubkey (33 bytes compressed or 65 uncompressed).
    pub pubkey: Vec<u8>,
    /// 64 bytes: r || s.
    pub signature: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VerifyInput {
    pub message: Vec<u8>,
    pub signers: Vec<SignerVerification>,
}

pub const SEEDS: &[[u8; 32]] = &[
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

pub fn keccak256(bytes: &[u8]) -> [u8; 32] {
    let mut h = Keccak::v256();
    let mut out = [0u8; 32];
    h.update(bytes);
    h.finalize(&mut out);
    out
}

/// Build a deterministic synthetic test vector with `num_signers` signers,
/// all signing the message `b"hello redstone"`.
pub fn make_test_vector(num_signers: usize) -> VerifyInput {
    assert!(
        num_signers <= SEEDS.len(),
        "only {} seeds available",
        SEEDS.len()
    );

    let message = b"hello redstone".to_vec();
    let digest = keccak256(&message);

    let signers = SEEDS
        .iter()
        .take(num_signers)
        .map(|seed| {
            let sk = SigningKey::from_bytes(seed.into()).expect("seed -> sk");
            let pubkey = sk.verifying_key().to_encoded_point(true).as_bytes().to_vec();

            let (sig, _recovery_id) = sk.sign_prehash_recoverable(&digest).expect("sign prehash");
            let signature = sig.to_bytes().to_vec();

            SignerVerification { pubkey, signature }
        })
        .collect();

    VerifyInput { message, signers }
}
