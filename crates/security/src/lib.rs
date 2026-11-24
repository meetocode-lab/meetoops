pub mod password;
pub mod rate_limit;
pub mod rbac;
pub mod token;
pub mod vault;

pub use password::{hash_password, validate_password_strength, verify_password, PasswordError};
pub use rate_limit::RateLimiter;
pub use rbac::{has_permission, permissions_for_role, Permission};
pub use token::{constant_time_eq, generate_csrf_token, generate_opaque_token, hash_token};
pub use vault::{EncryptedPayload, MasterKeyVault, VaultError};
