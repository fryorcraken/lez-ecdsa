use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VerifyInput {
    pub message: Vec<u8>,
    pub signature: Vec<u8>,
    pub expected_signer: [u8; 20],
}
