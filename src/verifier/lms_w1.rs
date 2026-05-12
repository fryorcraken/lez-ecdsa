use crate::{SignerVerification, VerifyInput};
use hbs_lms::{
    HssParameter, LmotsAlgorithm, LmsAlgorithm, Seed, Sha256_256, keygen, signature::SignerMut,
    verify,
};

fn hss_params() -> [HssParameter<Sha256_256>; 1] {
    [HssParameter::new(
        LmotsAlgorithm::LmotsW1,
        LmsAlgorithm::LmsH5,
    )]
}

pub fn sign(seed: &[u8; 32], message: &[u8]) -> SignerVerification {
    let lms_seed: Seed<Sha256_256> = (*seed).into();
    let (mut signing_key, verifying_key) =
        keygen::<Sha256_256>(&hss_params(), &lms_seed, None).expect("hbs-lms keygen");
    let signature = signing_key.try_sign(message).expect("hbs-lms sign");
    SignerVerification {
        pubkey: verifying_key.as_slice().to_vec(),
        signature: signature.as_ref().to_vec(),
    }
}

pub fn verify_all(input: &VerifyInput) -> Result<(), String> {
    for s in &input.signers {
        verify::<Sha256_256>(&input.message, &s.signature, &s.pubkey)
            .map_err(|e| format!("verify: {e}"))?;
    }
    Ok(())
}
