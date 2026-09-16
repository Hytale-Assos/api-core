use aes_gcm::aead::consts::U12;
use aes_gcm::aead::{Aead, Generate, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::prelude::{Engine as _, BASE64_STANDARD};
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::error::{AppError, AppResult};

const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;
const KEY_PEPPER_LABEL: &[u8] = b"api-core/key-hash-pepper";

type HmacSha256 = Hmac<Sha256>;
type GcmNonce = Nonce<U12>;

/// Chiffrement applicatif des données avant insertion en base.
///
/// - Les données sont chiffrées en AES-256-GCM (confidentialité + intégrité).
/// - Le format stocké est `base64(nonce || ciphertext || tag)`.
/// - La clé maître provient de la variable d'environnement `ENCRYPTION_KEY`
///   (base64 de 32 octets).
///
/// Les index d'authentification (recherche d'un module par API key) utilisent un
/// HMAC-SHA256 déterministe dérivé de la clé maître, afin de rester cherchables
/// sans jamais stocker la valeur en clair.
#[derive(Clone)]
pub struct Crypto {
    cipher: Aes256Gcm,
    pepper: [u8; KEY_LEN],
}

impl Crypto {
    /// Construit le module depuis la clé maître fournie en base64.
    pub fn from_base64(master_key_b64: &str) -> AppResult<Self> {
        let key = BASE64_STANDARD
            .decode(master_key_b64.trim())
            .map_err(|_| AppError::Internal(anyhow::anyhow!("ENCRYPTION_KEY n'est pas du base64")))?;

        if key.len() != KEY_LEN {
            return Err(AppError::Internal(anyhow::anyhow!(
                "ENCRYPTION_KEY doit faire {KEY_LEN} octets (reçu {}). Générer avec `openssl rand -base64 32`",
                key.len()
            )));
        }

        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|_| AppError::Internal(anyhow::anyhow!("clé de chiffrement invalide")))?;

        let mut mac = HmacSha256::new_from_slice(&key)
            .map_err(|_| AppError::Internal(anyhow::anyhow!("clé HMAC invalide")))?;
        mac.update(KEY_PEPPER_LABEL);
        let pepper_bytes = mac.finalize().into_bytes();

        let mut pepper = [0u8; KEY_LEN];
        pepper.copy_from_slice(&pepper_bytes);

        Ok(Self { cipher, pepper })
    }

    /// Chiffre une chaîne et renvoie sa représentation base64.
    pub fn encrypt_str(&self, plaintext: &str) -> AppResult<String> {
        let nonce = GcmNonce::generate();
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|_| AppError::Internal(anyhow::anyhow!("échec du chiffrement")))?;

        let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ciphertext);
        Ok(BASE64_STANDARD.encode(out))
    }

    /// Déchiffre une valeur produite par [`Crypto::encrypt_str`].
    pub fn decrypt_str(&self, encoded: &str) -> AppResult<String> {
        let raw = BASE64_STANDARD
            .decode(encoded)
            .map_err(|_| AppError::Internal(anyhow::anyhow!("valeur chiffrée invalide")))?;

        if raw.len() <= NONCE_LEN {
            return Err(AppError::Internal(anyhow::anyhow!(
                "valeur chiffrée tronquée"
            )));
        }

        let (nonce_bytes, ciphertext) = raw.split_at(NONCE_LEN);
        let nonce = GcmNonce::try_from(nonce_bytes)
            .map_err(|_| AppError::Internal(anyhow::anyhow!("nonce invalide")))?;
        let plaintext = self
            .cipher
            .decrypt(&nonce, ciphertext)
            .map_err(|_| AppError::Internal(anyhow::anyhow!("échec du déchiffrement")))?;

        String::from_utf8(plaintext)
            .map_err(|_| AppError::Internal(anyhow::anyhow!("données déchiffrées non UTF-8")))
    }

    /// Hachage déterministe (HMAC-SHA256) pour index de recherche sûr.
    pub fn keyed_hash(&self, value: &str) -> String {
        let mut mac = HmacSha256::new_from_slice(&self.pepper)
            .expect("la taille de la clé pepper est valide");
        mac.update(value.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Clé maître de 32 octets, encodée en base64 (valeur de test, jamais en prod).
    fn test_key() -> String {
        BASE64_STANDARD.encode([7u8; KEY_LEN])
    }

    fn crypto() -> Crypto {
        Crypto::from_base64(&test_key()).expect("clé de test valide")
    }

    #[test]
    fn rejects_invalid_base64() {
        assert!(Crypto::from_base64("!!! pas du base64 !!!").is_err());
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
        let plaintext = "secret-hmac-de-module";

        let encrypted = c.encrypt_str(plaintext).unwrap();
        assert_ne!(encrypted, plaintext, "le chiffré ne doit pas être en clair");
        assert_eq!(c.decrypt_str(&encrypted).unwrap(), plaintext);
    }

    #[test]
    fn encryption_is_non_deterministic() {
        let c = crypto();
        let a = c.encrypt_str("meme-valeur").unwrap();
        let b = c.encrypt_str("meme-valeur").unwrap();
        assert_ne!(a, b, "un nonce aléatoire doit changer le chiffré");
    }

    #[test]
    fn decrypting_with_another_key_fails() {
        let other = Crypto::from_base64(&BASE64_STANDARD.encode([9u8; KEY_LEN])).unwrap();
        let encrypted = crypto().encrypt_str("donnee").unwrap();
        assert!(other.decrypt_str(&encrypted).is_err());
    }

    #[test]
    fn tampering_with_ciphertext_fails() {
        let c = crypto();
        let encrypted = c.encrypt_str("donnee-integre").unwrap();

        let mut raw = BASE64_STANDARD.decode(&encrypted).unwrap();
        let last = raw.len() - 1;
        raw[last] ^= 0xff;
        let tampered = BASE64_STANDARD.encode(&raw);

        assert!(c.decrypt_str(&tampered).is_err(), "le tag GCM doit détecter l'altération");
    }

    #[test]
    fn rejects_truncated_and_non_base64_values() {
        let c = crypto();
        assert!(c.decrypt_str("pas du base64").is_err());
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
