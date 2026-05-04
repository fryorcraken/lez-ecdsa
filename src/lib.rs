use serde::{Deserialize, Serialize};
use tiny_keccak::{Hasher, Keccak};

pub mod verifier;

/// Signature schemes the bench compares.
///
/// Wire format: every fixture serializes to `VerifyInput`, whose `signers`
/// field is interpreted by the per-scheme guest binary picked at submission
/// time. The `Scheme` enum is host-side only — it tells the host which
/// guest ELF to embed and which signing path to use when building fixtures.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Scheme {
    EcdsaSecp256k1,
    SchnorrSecp256k1,
    Ed25519,
    EcdsaP256,
}

impl Scheme {
    pub const ALL: &'static [Scheme] = &[
        Scheme::EcdsaSecp256k1,
        Scheme::SchnorrSecp256k1,
        Scheme::Ed25519,
        Scheme::EcdsaP256,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            Scheme::EcdsaSecp256k1 => "ecdsa-secp256k1",
            Scheme::SchnorrSecp256k1 => "schnorr-secp256k1",
            Scheme::Ed25519 => "ed25519",
            Scheme::EcdsaP256 => "ecdsa-p256",
        }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        for scheme in Self::ALL {
            if scheme.slug() == s {
                return Ok(*scheme);
            }
        }
        Err(format!(
            "unknown scheme `{s}`; expected one of {:?}",
            Self::ALL.iter().map(|s| s.slug()).collect::<Vec<_>>()
        ))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SignerVerification {
    /// Scheme-specific pubkey encoding.
    pub pubkey: Vec<u8>,
    /// Scheme-specific signature encoding.
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

pub const MESSAGE: &[u8] = b"hello redstone";

pub fn keccak256(bytes: &[u8]) -> [u8; 32] {
    let mut h = Keccak::v256();
    let mut out = [0u8; 32];
    h.update(bytes);
    h.finalize(&mut out);
    out
}

/// Build a deterministic synthetic test vector for `scheme` with
/// `num_signers` signers, all signing `MESSAGE`.
pub fn make_test_vector(scheme: Scheme, num_signers: usize) -> VerifyInput {
    assert!(
        num_signers <= SEEDS.len(),
        "only {} seeds available",
        SEEDS.len()
    );

    let signers: Vec<SignerVerification> = SEEDS
        .iter()
        .take(num_signers)
        .map(|seed| match scheme {
            Scheme::EcdsaSecp256k1 => verifier::ecdsa_k256::sign(seed, MESSAGE),
            Scheme::SchnorrSecp256k1 => verifier::schnorr_k256::sign(seed, MESSAGE),
            Scheme::Ed25519 => verifier::ed25519::sign(seed, MESSAGE),
            Scheme::EcdsaP256 => verifier::ecdsa_p256::sign(seed, MESSAGE),
        })
        .collect();

    VerifyInput {
        message: MESSAGE.to_vec(),
        signers,
    }
}

/// Host-callable verify, mirroring the per-scheme guest verifier.
pub fn verify(scheme: Scheme, input: &VerifyInput) -> Result<(), String> {
    match scheme {
        Scheme::EcdsaSecp256k1 => verifier::ecdsa_k256::verify_all(input),
        Scheme::SchnorrSecp256k1 => verifier::schnorr_k256::verify_all(input),
        Scheme::Ed25519 => verifier::ed25519::verify_all(input),
        Scheme::EcdsaP256 => verifier::ecdsa_p256::verify_all(input),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Captured wire-format (bincode) hex of `make_test_vector(EcdsaSecp256k1, 1)`
    /// at the commit that introduced the Scheme enum. Locks the wire format so a
    /// future refactor of the builder cannot silently change what the
    /// guest sees on the wire.
    const ECDSA_SECP256K1_N1_BINCODE_HEX: &str = "0e0000000000000068656c6c6f2072656473746f6e65010000000000000021000000000000000202e3c3d6475bd9820f1b6966a40950b4475dbf7c39bcfc38e7bf1eae4194c4c840000000000000003ad850b0ac28fda79d349cfa96204eb72161ebd102b89c6c523b60ecb6b5e96e0d87b364eab41d3f7a3aee55f5cefab80b0e36fee6502e9053b5dee8d5f99c07";

    #[test]
    fn make_test_vector_ecdsa_secp256k1_n1_is_byte_stable() {
        let v = make_test_vector(Scheme::EcdsaSecp256k1, 1);
        let actual = hex::encode(bincode::serialize(&v).expect("bincode serialize"));
        assert_eq!(
            actual, ECDSA_SECP256K1_N1_BINCODE_HEX,
            "wire format for (ECDSA secp256k1, n=1) changed",
        );
    }

    fn round_trip(scheme: Scheme, n: usize) {
        let v = make_test_vector(scheme, n);
        verify(scheme, &v).unwrap_or_else(|e| panic!("{scheme:?} n={n} verify: {e}"));
    }

    fn flip_one_byte_fails(scheme: Scheme) {
        let mut v = make_test_vector(scheme, 1);
        v.signers[0].signature[0] ^= 0xff;
        assert!(
            verify(scheme, &v).is_err(),
            "{scheme:?}: one-byte sig flip should fail",
        );
    }

    #[test]
    fn round_trip_ecdsa_secp256k1() {
        round_trip(Scheme::EcdsaSecp256k1, 1);
        round_trip(Scheme::EcdsaSecp256k1, 3);
    }

    #[test]
    fn round_trip_schnorr_secp256k1() {
        round_trip(Scheme::SchnorrSecp256k1, 1);
        round_trip(Scheme::SchnorrSecp256k1, 3);
    }

    #[test]
    fn round_trip_ed25519() {
        round_trip(Scheme::Ed25519, 1);
        round_trip(Scheme::Ed25519, 3);
    }

    #[test]
    fn round_trip_ecdsa_p256() {
        round_trip(Scheme::EcdsaP256, 1);
        round_trip(Scheme::EcdsaP256, 3);
    }

    #[test]
    fn negative_ecdsa_secp256k1() {
        flip_one_byte_fails(Scheme::EcdsaSecp256k1);
    }

    #[test]
    fn negative_schnorr_secp256k1() {
        flip_one_byte_fails(Scheme::SchnorrSecp256k1);
    }

    #[test]
    fn negative_ed25519() {
        flip_one_byte_fails(Scheme::Ed25519);
    }

    #[test]
    fn negative_ecdsa_p256() {
        flip_one_byte_fails(Scheme::EcdsaP256);
    }
}
