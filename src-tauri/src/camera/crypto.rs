use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce
};
use sha2::{Sha256, Digest};
use base64::prelude::*;

fn get_machine_key() -> [u8; 32] {
    let mut hasher = Sha256::new();
    let hostname = std::env::var("HOSTNAME").unwrap_or_else(|_| "onliview-host".to_string());
    let user = std::env::var("USER").unwrap_or_else(|_| "onliview-user".to_string());
    let machine_id = std::fs::read_to_string("/etc/machine-id").unwrap_or_else(|_| "onliview-static-salt-v1".to_string());
    
    hasher.update(b"ONLIVIEW_KEY_V1_");
    hasher.update(hostname.as_bytes());
    hasher.update(user.as_bytes());
    hasher.update(machine_id.trim().as_bytes());
    
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result[0..32]);
    key
}

pub fn encrypt_password(plain: &str) -> Result<String, String> {
    if plain.is_empty() {
        return Ok(String::new());
    }
    let key = get_machine_key();
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
    
    // Generate deterministic or pseudo-random 12-byte nonce
    let nonce_bytes = [0x4f, 0x6e, 0x6c, 0x69, 0x56, 0x69, 0x65, 0x77, 0x56, 0x4d, 0x53, 0x01]; // "OnliViewVMS\x01"
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    let encrypted = cipher.encrypt(nonce, plain.as_bytes()).map_err(|e| e.to_string())?;
    Ok(BASE64_STANDARD.encode(encrypted))
}

pub fn decrypt_password(encrypted_b64: &str) -> Result<String, String> {
    if encrypted_b64.is_empty() {
        return Ok(String::new());
    }
    let encrypted = BASE64_STANDARD.decode(encrypted_b64).map_err(|e| e.to_string())?;
    let key = get_machine_key();
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
    
    let nonce_bytes = [0x4f, 0x6e, 0x6c, 0x69, 0x56, 0x69, 0x65, 0x77, 0x56, 0x4d, 0x53, 0x01];
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    let decrypted = cipher.decrypt(nonce, encrypted.as_ref()).map_err(|e| e.to_string())?;
    String::from_utf8(decrypted).map_err(|e| e.to_string())
}
