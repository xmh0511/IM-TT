use bcrypt::{hash, verify, DEFAULT_COST};
use anyhow::Result;

pub fn hash_password(password: &str) -> Result<String> {
    let hashed = hash(password, DEFAULT_COST)?;
    Ok(hashed)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let valid = verify(password, hash)?;
    Ok(valid)
}
