use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// Generates a 256-bit CSPRNG URL-safe opaque token (for sessions, registration tokens, API tokens).
pub fn generate_opaque_token() -> String {
    let mut buf = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}

/// Generates a 256-bit CSRF token.
pub fn generate_csrf_token() -> String {
    generate_opaque_token()
}

/// Computes SHA-256 hex digest of an opaque token for safe database storage.
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

/// Constant-time string equality check (for CSRF and token comparisons).
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    a.as_bytes().ct_eq(b.as_bytes()).into()
}
