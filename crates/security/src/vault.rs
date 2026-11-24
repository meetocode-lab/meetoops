use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use thiserror::Error;
use zeroize::Zeroize;

#[derive(Debug, Error)]
pub enum VaultError {
    #[error("failed to read or initialize master key: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid master key format: {0}")]
    InvalidKey(String),
    #[error("master key mismatch: key fingerprint does not match initialized database")]
    KeyMismatch,
    #[error("encryption failed")]
    EncryptFailed,
    #[error("decryption failed (tampered ciphertext or wrong key/AAD)")]
    DecryptFailed,
}

#[derive(Debug, Clone)]
pub struct EncryptedPayload {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
}

/// Holds the 256-bit AES-GCM master key in memory with zeroize-on-drop and redacted Debug output.
pub struct MasterKeyVault {
    key_bytes: [u8; 32],
    fingerprint: String,
}

impl std::fmt::Debug for MasterKeyVault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MasterKeyVault")
            .field("key_bytes", &"[REDACTED]")
            .field("fingerprint", &self.fingerprint)
            .finish()
    }
}

impl Drop for MasterKeyVault {
    fn drop(&mut self) {
        self.key_bytes.zeroize();
    }
}

impl MasterKeyVault {
    /// Loads an existing master key from `path`, or atomically generates a new 256-bit key
    /// if `path` does not exist yet (zero-config first-run behavior).
    pub fn load_or_create(path: &Path) -> Result<Self, VaultError> {
        if path.exists() {
            let content = fs::read_to_string(path)?;
            let decoded = BASE64
                .decode(content.trim())
                .map_err(|e| VaultError::InvalidKey(e.to_string()))?;
            if decoded.len() != 32 {
                return Err(VaultError::InvalidKey(format!(
                    "expected 32 bytes, found {}",
                    decoded.len()
                )));
            }
            let mut key_bytes = [0u8; 32];
            key_bytes.copy_from_slice(&decoded);
            Ok(Self::from_bytes(key_bytes))
        } else {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut key_bytes = [0u8; 32];
            rand::rngs::OsRng.fill_bytes(&mut key_bytes);
            let encoded = BASE64.encode(key_bytes);

            fs::write(path, format!("{encoded}\n"))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
            }
            Ok(Self::from_bytes(key_bytes))
        }
    }

    pub fn from_bytes(key_bytes: [u8; 32]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(key_bytes);
        let fingerprint = hex::encode(hasher.finalize());
        Self {
            key_bytes,
            fingerprint,
        }
    }

    /// SHA-256 hex fingerprint of the master key (safe to store in DB and display truncated in Settings).
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    /// Encrypts plaintext using AES-256-GCM bound to Associated Authenticated Data (e.g. `scope:reference_id`).
    pub fn encrypt(&self, plaintext: &[u8], aad: &str) -> Result<EncryptedPayload, VaultError> {
        let cipher = Aes256Gcm::new_from_slice(&self.key_bytes)
            .map_err(|_| VaultError::EncryptFailed)?;
        let mut nonce_bytes = [0u8; 12];
        rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(
                nonce,
                Payload {
                    msg: plaintext,
                    aad: aad.as_bytes(),
                },
            )
            .map_err(|_| VaultError::EncryptFailed)?;

        Ok(EncryptedPayload {
            ciphertext,
            nonce: nonce_bytes.to_vec(),
        })
    }

    /// Decrypts AES-256-GCM ciphertext verifying both tag integrity and AAD binding.
    pub fn decrypt(
        &self,
        ciphertext: &[u8],
        nonce_bytes: &[u8],
        aad: &str,
    ) -> Result<Vec<u8>, VaultError> {
        if nonce_bytes.len() != 12 {
            return Err(VaultError::DecryptFailed);
        }
        let cipher = Aes256Gcm::new_from_slice(&self.key_bytes)
            .map_err(|_| VaultError::DecryptFailed)?;
        let nonce = Nonce::from_slice(nonce_bytes);

        cipher
            .decrypt(
                nonce,
                Payload {
                    msg: ciphertext,
                    aad: aad.as_bytes(),
                },
            )
            .map_err(|_| VaultError::DecryptFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypts_decrypts_and_detects_tampering() {
        let vault = MasterKeyVault::from_bytes([42u8; 32]);
        let aad = "service_connection:svc-123";
        let secret = b"postgres://admin:supersecret@172.18.0.2:5432/prod";

        let enc = vault.encrypt(secret, aad).expect("encrypt");
        let dec = vault
            .decrypt(&enc.ciphertext, &enc.nonce, aad)
            .expect("decrypt");
        assert_eq!(dec, secret);

        // Wrong AAD must fail
        assert!(vault
            .decrypt(&enc.ciphertext, &enc.nonce, "service_connection:svc-999")
            .is_err());

        // Tampered ciphertext must fail
        let mut tampered = enc.ciphertext.clone();
        tampered[0] ^= 0xFF;
        assert!(vault.decrypt(&tampered, &enc.nonce, aad).is_err());
    }
}
