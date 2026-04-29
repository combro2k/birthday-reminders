use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};

/// Validate password strength. Returns an error message if the password is too weak.
pub fn validate_password(password: &str) -> Result<(), &'static str> {
    if password.len() < 8 {
        return Err("Password must be at least 8 characters");
    }
    if !password.chars().any(|c| c.is_uppercase()) {
        return Err("Password must contain at least one uppercase letter");
    }
    if !password.chars().any(|c| c.is_lowercase()) {
        return Err("Password must contain at least one lowercase letter");
    }
    if !password.chars().any(|c| c.is_ascii_digit()) {
        return Err("Password must contain at least one digit");
    }
    Ok(())
}

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_short_password() {
        assert!(validate_password("Ab1").is_err());
    }

    #[test]
    fn validate_rejects_no_uppercase() {
        assert!(validate_password("abcdefg1").is_err());
    }

    #[test]
    fn validate_rejects_no_lowercase() {
        assert!(validate_password("ABCDEFG1").is_err());
    }

    #[test]
    fn validate_rejects_no_digit() {
        assert!(validate_password("Abcdefgh").is_err());
    }

    #[test]
    fn validate_accepts_strong_password() {
        assert!(validate_password("Abcdefg1").is_ok());
    }

    #[test]
    fn hash_and_verify_roundtrip() {
        let password = "TestPass1";
        let hash = hash_password(password).unwrap();
        assert!(verify_password(password, &hash));
    }

    #[test]
    fn verify_wrong_password_fails() {
        let hash = hash_password("TestPass1").unwrap();
        assert!(!verify_password("WrongPass1", &hash));
    }

    #[test]
    fn verify_invalid_hash_returns_false() {
        assert!(!verify_password("anything", "not-a-valid-hash"));
    }
}
