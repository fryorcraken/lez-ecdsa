use k256::ecdsa::{Signature, VerifyingKey, signature::hazmat::PrehashVerifier};
use nssa_core::program::{AccountPostState, ProgramInput, ProgramOutput, read_nssa_inputs};
use serde::{Deserialize, Serialize};
use tiny_keccak::{Hasher, Keccak};

#[derive(Serialize, Deserialize)]
struct SignerVerification {
    pubkey: Vec<u8>,
    signature: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
struct VerifyInput {
    message: Vec<u8>,
    signers: Vec<SignerVerification>,
}

fn keccak256(bytes: &[u8]) -> [u8; 32] {
    let mut h = Keccak::v256();
    let mut out = [0u8; 32];
    h.update(bytes);
    h.finalize(&mut out);
    out
}

fn main() {
    let (
        ProgramInput {
            self_program_id,
            caller_program_id,
            pre_states,
            instruction,
        },
        instruction_data,
    ) = read_nssa_inputs::<VerifyInput>();

    let digest = keccak256(&instruction.message);
    for s in &instruction.signers {
        let vk = VerifyingKey::from_sec1_bytes(&s.pubkey).expect("bad pubkey");
        let sig = Signature::from_slice(&s.signature).expect("bad sig");
        vk.verify_prehash(&digest, &sig).expect("verify failed");
    }

    // Pass-through: verifier doesn't mutate any account.
    let post_states: Vec<AccountPostState> = pre_states
        .iter()
        .map(|awm| AccountPostState::new(awm.account.clone()))
        .collect();

    ProgramOutput::new(
        self_program_id,
        caller_program_id,
        instruction_data,
        pre_states,
        post_states,
    )
    .write();
}
