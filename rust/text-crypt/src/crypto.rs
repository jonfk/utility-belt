use base64;
use chacha20poly1305::aead::{Aead, NewAead};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use thiserror::Error;

use crate::CryptBlock;
use crate::BASE64_CONFIG;

pub fn encrypt(password: &str, contents: &str) -> CryptBlock {
    let key = Key::from_slice(password.as_bytes()); // 32-bytes
    let cipher = ChaCha20Poly1305::new(key);

    let nonce_str = "unique nonce";
    let nonce = Nonce::from_slice(nonce_str.as_bytes()); // 12-bytes; unique per message

    let ciphertext_bytes = cipher
        .encrypt(nonce, contents.as_bytes().as_ref())
        .expect("encryption failure!"); // NOTE: handle this error to avoid panics!

    let ciphertext = base64::encode_config(ciphertext_bytes, BASE64_CONFIG);
    CryptBlock {
        algorithm: Some("ChaCha20Poly1305".to_string()),
        nonce: Some(nonce_str.to_string()),
        ciphertext,
    }
}

pub fn decrypt(password: &str, encrypted: &CryptBlock) -> String {
    let key = Key::from_slice(password.as_bytes()); // 32-bytes
    let cipher = ChaCha20Poly1305::new(key);

    let nonce = Nonce::from_slice(encrypted.nonce.as_ref().expect("missing nonce").as_bytes()); // 12-bytes; unique per message

    let ciphertext_bytes =
        base64::decode_config(&encrypted.ciphertext, BASE64_CONFIG).expect("failed base64 decode");

    let plaintext = cipher
        .decrypt(nonce, ciphertext_bytes.as_ref())
        .expect("decryption failure!"); // NOTE: handle this error to avoid panics!

    String::from_utf8(plaintext).expect("failed decoding plaintext bytes to utf8")
}
