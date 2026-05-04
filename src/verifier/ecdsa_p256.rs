use crate::{SignerVerification, VerifyInput};
use p256::ecdsa::{Signature, SigningKey, VerifyingKey, signature::hazmat::PrehashVerifier};
use sha2::{Digest, Sha256};

fn prehash(message: &[u8]) -> [u8; 32] {
    Sha256::digest(message).into()
}

pub fn sign(seed: &[u8; 32], message: &[u8]) -> SignerVerification {
    let sk = SigningKey::from_bytes(seed.into()).expect("seed -> sk");
    let pubkey = sk
        .verifying_key()
        .to_encoded_point(true)
        .as_bytes()
        .to_vec();

    let digest = prehash(message);
    let (sig, _) = sk.sign_prehash_recoverable(&digest).expect("sign prehash");
    let signature = sig.to_bytes().to_vec();

    SignerVerification { pubkey, signature }
}

pub fn verify_all(input: &VerifyInput) -> Result<(), String> {
    let digest = prehash(&input.message);
    for s in &input.signers {
        let vk = VerifyingKey::from_sec1_bytes(&s.pubkey).map_err(|e| format!("pubkey: {e}"))?;
        let sig = Signature::from_slice(&s.signature).map_err(|e| format!("sig: {e}"))?;
        vk.verify_prehash(&digest, &sig)
            .map_err(|e| format!("verify: {e}"))?;
    }
    Ok(())
}
