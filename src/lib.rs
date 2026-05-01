use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VerifyOutput {
    pub valid_count: u32,
    pub all_valid: bool,
}
