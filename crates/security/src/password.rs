use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Algorithm, Argon2, Params, Version,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PasswordError {
    #[error("password policy violation: {0}")]
    Policy(String),
    #[error("password hashing failed")]
    HashingFailed,
}

const WEAK_PASSWORDS: &[&str] = &[
    "password1234",
    "123456789012",
    "admin1234567",
    "meetoops1234",
    "qwertyuiop12",
];

/// Enforces minimum password policy (12+ chars, upper, lower, digit).
pub fn validate_password_strength(password: &str) -> Result<(), PasswordError> {
    if password.len() < 12 {
        return Err(PasswordError::Policy(
            "Password must be at least 12 characters long.".to_string(),
        ));
    }
    if password.len() > 128 {
        return Err(PasswordError::Policy(
            "Password must be at most 128 characters long.".to_string(),
        ));
    }
    let has_upper = password.chars().any(|c| c.is_ascii_uppercase());
    let has_lower = password.chars().any(|c| c.is_ascii_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());

    if !(has_upper && has_lower && has_digit) {
        return Err(PasswordError::Policy(
            "Password must contain uppercase, lowercase, and numeric characters.".to_string(),
        ));
    }

    let lowered = password.to_ascii_lowercase();
    if WEAK_PASSWORDS.iter().any(|w| lowered == *w) {
        return Err(PasswordError::Policy(
            "Password is too common. Choose a stronger passphrase.".to_string(),
        ));
    }
    Ok(())
}

fn argon2_instance() -> Argon2<'static> {
    // OWASP-aligned Argon2id parameters (19 MiB memory, 2 iterations, 1 lane)
    let params = Params::new(19 * 1024, 2, 1, None).expect("valid argon2 params");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

/// Hashes a password with Argon2id on a blocking thread so async workers are never stalled.
pub async fn hash_password(password: String) -> Result<String, PasswordError> {
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        argon2_instance()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|_| PasswordError::HashingFailed)
    })
    .await
    .map_err(|_| PasswordError::HashingFailed)?
}

/// Verifies a plaintext password against a stored Argon2id PHC hash on a blocking thread.
pub async fn verify_password(password: String, hash: String) -> Result<bool, PasswordError> {
    tokio::task::spawn_blocking(move || {
        let parsed = PasswordHash::new(&hash).map_err(|_| PasswordError::HashingFailed)?;
        Ok(argon2_instance()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok())
    })
    .await
    .map_err(|_| PasswordError::HashingFailed)?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn hashes_and_verifies_argon2id() {
        let pw = "WarmObsidian2026!".to_string();
        assert!(validate_password_strength(&pw).is_ok());
        let hashed = hash_password(pw.clone()).await.expect("hash");
        assert!(hashed.starts_with("$argon2id$"));
        assert!(verify_password(pw, hashed.clone()).await.expect("verify"));
        assert!(!verify_password("WrongPassword123".to_string(), hashed)
            .await
            .expect("verify wrong"));
    }

    #[test]
    fn rejects_weak_passwords() {
        assert!(validate_password_strength("short1A").is_err());
        assert!(validate_password_strength("alllowercase123").is_err());
        assert!(validate_password_strength("NoDigitsAtAllHere").is_err());
    }
}
