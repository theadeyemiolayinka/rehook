//! Argon2id password hashing and verification.

use anyhow::Result;

/// Hash a password using Argon2id.
pub fn hash_password(password: &str) -> Result<String> {
    use argon2::password_hash::SaltString;
    use argon2::{Algorithm, Argon2, Params, PasswordHasher, Version};
    let salt = SaltString::generate(&mut rand::thread_rng());
    let params =
        Params::new(19 * 1024, 2, 1, None).map_err(|e| anyhow::anyhow!("argon2 params: {e}"))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("hashing password: {e}"))?
        .to_string();
    Ok(hash)
}

/// Verify a password against an Argon2id hash in constant time.
pub fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::PasswordVerifier;
    let parsed = match argon2::PasswordHash::new(hash) {
        Ok(p) => p,
        Err(_) => return false,
    };
    let argon2 = argon2::Argon2::default();
    matches!(argon2.verify_password(password.as_bytes(), &parsed), Ok(()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify() {
        let hash = hash_password("testpass123").unwrap();
        assert!(verify_password("testpass123", &hash));
        assert!(!verify_password("wrong", &hash));
    }
}
