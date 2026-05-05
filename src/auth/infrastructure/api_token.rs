use rand::Rng;
use sha2::{Digest, Sha256};

/// Generate a random API token and return (plain_token, hashed_token)
pub fn generate_api_token() -> (String, String) {
    let mut rng = rand::thread_rng();
    let token_bytes: [u8; 32] = rng.r#gen();
    let plain = format!("br_{}", hex::encode(token_bytes));
    let hash = hash_token(&plain);
    (plain, hash)
}

/// Hash a token for storage
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}
