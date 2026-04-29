use chacha20poly1305::{
    XChaCha20Poly1305,
    aead::{Aead, KeyInit},
};
use rand::RngCore;
use sha2::{Digest, Sha256};

/// Derive a 256-bit key from the session secret using SHA-256.
fn derive_key(secret: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hasher.finalize().into()
}

/// Encrypt a plaintext string. Returns a base64-encoded string containing nonce + ciphertext.
pub fn encrypt(plaintext: &str, secret: &str) -> anyhow::Result<String> {
    let key = derive_key(secret);
    let cipher = XChaCha20Poly1305::new_from_slice(&key)
        .map_err(|e| anyhow::anyhow!("Failed to create cipher: {}", e))?;

    let mut nonce_bytes = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = nonce_bytes.into();

    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

    // Prepend nonce to ciphertext
    let mut combined = nonce_bytes.to_vec();
    combined.extend_from_slice(&ciphertext);

    Ok(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &combined,
    ))
}

/// Decrypt a base64-encoded string (nonce + ciphertext) back to plaintext.
pub fn decrypt(encrypted: &str, secret: &str) -> anyhow::Result<String> {
    let key = derive_key(secret);
    let cipher = XChaCha20Poly1305::new_from_slice(&key)
        .map_err(|e| anyhow::anyhow!("Failed to create cipher: {}", e))?;

    let combined = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encrypted)
        .map_err(|e| anyhow::anyhow!("Base64 decode failed: {}", e))?;

    if combined.len() < 24 {
        anyhow::bail!("Encrypted data too short");
    }

    let (nonce_bytes, ciphertext) = combined.split_at(24);
    let nonce: [u8; 24] = nonce_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("Invalid nonce length"))?;

    let plaintext = cipher
        .decrypt(&nonce.into(), ciphertext)
        .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;

    String::from_utf8(plaintext)
        .map_err(|e| anyhow::anyhow!("Decrypted data is not valid UTF-8: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let secret = "test-secret-that-is-at-least-32-chars-long!!";
        let plaintext = "my-super-secret-password";

        let encrypted = encrypt(plaintext, secret).unwrap();
        assert_ne!(encrypted, plaintext);

        let decrypted = decrypt(&encrypted, secret).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_wrong_key_fails() {
        let secret = "test-secret-that-is-at-least-32-chars-long!!";
        let wrong = "wrong-secret-that-is-at-least-32-chars-long!!";
        let plaintext = "my-super-secret-password";

        let encrypted = encrypt(plaintext, secret).unwrap();
        assert!(decrypt(&encrypted, wrong).is_err());
    }

    #[test]
    fn test_different_encryptions_differ() {
        let secret = "test-secret-that-is-at-least-32-chars-long!!";
        let plaintext = "same-text";

        let enc1 = encrypt(plaintext, secret).unwrap();
        let enc2 = encrypt(plaintext, secret).unwrap();
        // Different nonces produce different ciphertexts
        assert_ne!(enc1, enc2);

        // Both decrypt correctly
        assert_eq!(decrypt(&enc1, secret).unwrap(), plaintext);
        assert_eq!(decrypt(&enc2, secret).unwrap(), plaintext);
    }
}
