use std::string::FromUtf8Error;

use base64;
use chacha20poly1305::aead::{self, Aead, NewAead};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use thiserror::Error;

use crate::{CryptBlock, BASE64_CONFIG};

pub fn encrypt(password: &str, contents: &str) -> Result<CryptBlock, CryptoEncryptError> {
    let key = Key::from_slice(password.as_bytes()); // 32-bytes
    let cipher = ChaCha20Poly1305::new(key);

    let nonce_bytes = generate_random_nonce();
    let nonce = Nonce::from_slice(&nonce_bytes); // 12-bytes; unique per message

    let ciphertext_bytes = cipher
        .encrypt(nonce, contents.as_bytes().as_ref())
        .map_err(|e| CryptoEncryptError::Encryption(e))?;

    let ciphertext = base64::encode_config(ciphertext_bytes, BASE64_CONFIG);
    Ok(CryptBlock {
        algorithm: Some("ChaCha20Poly1305".to_string()),
        nonce: Some(base64::encode_config(nonce_bytes, BASE64_CONFIG)),
        ciphertext,
    })
}

fn generate_random_nonce() -> [u8; 12] {
    use rand::prelude::*;
    use rand_chacha::ChaCha20Rng;

    let mut rng = ChaCha20Rng::from_entropy();
    let mut nonce: [u8; 12] = [0; 12];
    rng.fill(&mut nonce);
    nonce
}

pub fn decrypt(password: &str, encrypted: &CryptBlock) -> Result<String, CryptoDecryptError> {
    let key = Key::from_slice(password.as_bytes()); // 32-bytes
    let cipher = ChaCha20Poly1305::new(key);

    let nonce_bytes = base64::decode_config(
        &encrypted
            .nonce
            .as_ref()
            .ok_or(CryptoDecryptError::MissingNonce)?,
        BASE64_CONFIG,
    )
    .map_err(|e| CryptoDecryptError::Base64Decode("nonce".to_string(), e))?;

    let nonce = Nonce::from_slice(&nonce_bytes); // 12-bytes; unique per message

    let ciphertext_bytes = base64::decode_config(&encrypted.ciphertext, BASE64_CONFIG)
        .map_err(|e| CryptoDecryptError::Base64Decode("ciphertext".to_string(), e))?;

    let plaintext = cipher
        .decrypt(nonce, ciphertext_bytes.as_ref())
        .map_err(|e| CryptoDecryptError::Decryption(e))?;

    Ok(String::from_utf8(plaintext).map_err(|e| CryptoDecryptError::Utf8FromBytes(e))?)
}

#[derive(Error, Debug)]
pub enum CryptoEncryptError {
    #[error("Failed encryption: {}", .0)]
    Encryption(aead::Error),
}

#[derive(Error, Debug)]
pub enum CryptoDecryptError {
    #[error("Missing Nonce")]
    MissingNonce,

    #[error("Error Base64 decoding {}: {}", .0, .1)]
    Base64Decode(String, base64::DecodeError),

    #[error("Failed decryption: {}", .0)]
    Decryption(aead::Error),

    #[error("Failed parsing utf-8 from decrypted bytes: {}", .0)]
    Utf8FromBytes(FromUtf8Error),
}
