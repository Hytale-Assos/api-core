use aes_gcm::aead::consts::U12;
use aes_gcm::aead::{Aead, Generate, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::prelude::{BASE64_STANDARD, Engine as _};
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::error::{AppError, AppResult};

const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;
const KEY_PEPPER_LABEL: &[u8] = b"api-core/key-hash-pepper";

type HmacSha256 = Hmac<Sha256>;
type GcmNonce = Nonce<U12>;

/// Application-level encryption of data before insertion.
///
/// - Data is encrypted with AES-256-GCM (confidentiality + integrity).
/// - Stored format is `base64(nonce || ciphertext || tag)`.
/// - The master key comes from the `ENCRYPTION_KEY` environment variable
///   (base64 of 32 bytes).
///
/// Authentication indexes (looking up a module by API key) use a deterministic
/// HMAC-SHA256 derived from the master key, so they stay searchable without
/// ever storing the value in the clear.
#[derive(Clone)]
pub struct Crypto {
    cipher: Aes256Gcm,
    pepper: [u8; KEY_LEN],
}

impl Crypto {
    /// Builds the module from the base64 master key.
    pub fn from_base64(master_key_b64: &str) -> AppResult<Self> {
        let key = BASE64_STANDARD
            .decode(master_key_b64.trim())
            .map_err(|_| AppError::Internal(anyhow::anyhow!("ENCRYPTION_KEY is not base64")))?;

        if key.len() != KEY_LEN {
            return Err(AppError::Internal(anyhow::anyhow!(
                "ENCRYPTION_KEY must be {KEY_LEN} bytes (got {}). Generate with `openssl rand -base64 32`",
                key.len()
            )));
        }

        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|_| AppError::Internal(anyhow::anyhow!("invalid encryption key")))?;

        let mut mac = HmacSha256::new_from_slice(&key)
            .map_err(|_| AppError::Internal(anyhow::anyhow!("invalid HMAC key")))?;
        mac.update(KEY_PEPPER_LABEL);
        let pepper_bytes = mac.finalize().into_bytes();

        let mut pepper = [0u8; KEY_LEN];
        pepper.copy_from_slice(&pepper_bytes);

        Ok(Self { cipher, pepper })
    }

    /// Encrypts a string and returns its base64 representation.
    pub fn encrypt_str(&self, plaintext: &str) -> AppResult<String> {
        let nonce = GcmNonce::generate();
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|_| AppError::Internal(anyhow::anyhow!("encryption failed")))?;

        let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ciphertext);
        Ok(BASE64_STANDARD.encode(out))
    }

    /// Decrypts a value produced by [`Crypto::encrypt_str`].
    pub fn decrypt_str(&self, encoded: &str) -> AppResult<String> {
        let raw = BASE64_STANDARD
            .decode(encoded)
            .map_err(|_| AppError::Internal(anyhow::anyhow!("invalid ciphertext")))?;

        if raw.len() <= NONCE_LEN {
            return Err(AppError::Internal(anyhow::anyhow!("truncated ciphertext")));
        }

        let (nonce_bytes, ciphertext) = raw.split_at(NONCE_LEN);
        let nonce = GcmNonce::try_from(nonce_bytes)
            .map_err(|_| AppError::Internal(anyhow::anyhow!("invalid nonce")))?;
        let plaintext = self
            .cipher
            .decrypt(&nonce, ciphertext)
            .map_err(|_| AppError::Internal(anyhow::anyhow!("decryption failed")))?;

        String::from_utf8(plaintext)
            .map_err(|_| AppError::Internal(anyhow::anyhow!("decrypted data is not UTF-8")))
    }

    /// Deterministic hash (HMAC-SHA256) for a safe search index.
    pub fn keyed_hash(&self, value: &str) -> String {
        let mut mac =
            HmacSha256::new_from_slice(&self.pepper).expect("the pepper key length is valid");
        mac.update(value.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 32-byte master key, base64-encoded (test value, never used in prod).
    fn test_key() -> String {
        BASE64_STANDARD.encode([7u8; KEY_LEN])
    }

    fn crypto() -> Crypto {
        Crypto::from_base64(&test_key()).expect("valid test key")
    }

    #[test]
    fn rejects_invalid_base64() {
        assert!(Crypto::from_base64("!!! not base64 !!!").is_err());
    }

    #[test]
    fn rejects_wrong_key_length() {
        let short = BASE64_STANDARD.encode([0u8; 16]);
        assert!(Crypto::from_base64(&short).is_err());
    }

    #[test]
    fn accepts_valid_key_and_trims_whitespace() {
        let padded = format!("  {}\n", test_key());
        assert!(Crypto::from_base64(&padded).is_ok());
    }

    #[test]
    fn encrypt_then_decrypt_roundtrip() {
        let c = crypto();
        let plaintext = "module-hmac-secret";

        let encrypted = c.encrypt_str(plaintext).unwrap();
        assert_ne!(encrypted, plaintext, "ciphertext must not be plaintext");
        assert_eq!(c.decrypt_str(&encrypted).unwrap(), plaintext);
    }

    #[test]
    fn encryption_is_non_deterministic() {
        let c = crypto();
        let a = c.encrypt_str("same-value").unwrap();
        let b = c.encrypt_str("same-value").unwrap();
        assert_ne!(a, b, "a random nonce must change the ciphertext");
    }

    #[test]
    fn decrypting_with_another_key_fails() {
        let other = Crypto::from_base64(&BASE64_STANDARD.encode([9u8; KEY_LEN])).unwrap();
        let encrypted = crypto().encrypt_str("data").unwrap();
        assert!(other.decrypt_str(&encrypted).is_err());
    }

    #[test]
    fn tampering_with_ciphertext_fails() {
        let c = crypto();
        let encrypted = c.encrypt_str("integrity-protected-data").unwrap();

        let mut raw = BASE64_STANDARD.decode(&encrypted).unwrap();
        let last = raw.len() - 1;
        raw[last] ^= 0xff;
        let tampered = BASE64_STANDARD.encode(&raw);

        assert!(
            c.decrypt_str(&tampered).is_err(),
            "the GCM tag must detect tampering"
        );
    }

    #[test]
    fn rejects_truncated_and_non_base64_values() {
        let c = crypto();
        assert!(c.decrypt_str("not base64").is_err());
        let too_short = BASE64_STANDARD.encode([0u8; NONCE_LEN]);
        assert!(c.decrypt_str(&too_short).is_err());
    }

    #[test]
    fn keyed_hash_is_deterministic_and_differs_by_value() {
        let c = crypto();
        assert_eq!(c.keyed_hash("api-key-1"), c.keyed_hash("api-key-1"));
        assert_ne!(c.keyed_hash("api-key-1"), c.keyed_hash("api-key-2"));
    }

    #[test]
    fn keyed_hash_depends_on_master_key() {
        let other = Crypto::from_base64(&BASE64_STANDARD.encode([9u8; KEY_LEN])).unwrap();
        assert_ne!(crypto().keyed_hash("api-key"), other.keyed_hash("api-key"));
    }
}
