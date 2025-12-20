//! PKCE (Proof Key for Code Exchange) implementation

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::Rng;
use sha2::{Digest, Sha256};

use crate::common::OAuthFlowState;

/// Generate a random code verifier (43-128 characters)
pub fn generate_code_verifier() -> String {
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..64).map(|_| rng.r#gen()).collect();
    URL_SAFE_NO_PAD.encode(&bytes)
}

/// Generate code challenge from verifier using S256 method
pub fn generate_code_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    URL_SAFE_NO_PAD.encode(hash)
}

/// Generate a random state parameter
pub fn generate_state() -> String {
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.r#gen()).collect();
    URL_SAFE_NO_PAD.encode(&bytes)
}

/// Create a new PKCE flow state
pub fn create_pkce_state() -> OAuthFlowState {
    let state = generate_state();
    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);

    OAuthFlowState::new(state, code_verifier, code_challenge)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_verifier_length() {
        let verifier = generate_code_verifier();
        assert!(verifier.len() >= 43 && verifier.len() <= 128);
    }

    #[test]
    fn test_code_challenge_is_sha256() {
        let verifier = "test_verifier_string";
        let challenge = generate_code_challenge(verifier);

        // SHA256 produces 32 bytes, base64url encoded = 43 chars
        assert_eq!(challenge.len(), 43);
    }

    #[test]
    fn test_state_uniqueness() {
        let state1 = generate_state();
        let state2 = generate_state();
        assert_ne!(state1, state2);
    }

    #[test]
    fn test_pkce_state_creation() {
        let flow_state = create_pkce_state();
        assert!(!flow_state.state.is_empty());
        assert!(!flow_state.code_verifier.is_empty());
        assert!(!flow_state.code_challenge.is_empty());
        assert!(flow_state.redirect_uri.is_none());
    }
}
