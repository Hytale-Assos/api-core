use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Verifies a signature using a constant-time comparison.
pub fn verify(secret: &str, timestamp: &str, body: &[u8], provided_hex: &str) -> bool {
    let provided = match hex::decode(provided_hex.trim()) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC accepts a key of arbitrary size");
    mac.update(timestamp.as_bytes());
    mac.update(b".");
    mac.update(body);

    mac.verify_slice(&provided).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Computes the expected HMAC-SHA256 signature for a request.
    ///
    /// The signed message is `"{timestamp}.{body}"`. Serves as a reference to
    /// check `verify` (modules generate their own signature).
    fn sign(secret: &str, timestamp: &str, body: &[u8]) -> String {
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .expect("HMAC accepts a key of arbitrary size");
        mac.update(timestamp.as_bytes());
        mac.update(b".");
        mac.update(body);
        hex::encode(mac.finalize().into_bytes())
    }

    #[test]
    fn verify_accepts_matching_signature() {
        let signature = sign(
            "s3cr3t",
            "1700000000",
            br#"{"event_type":"session_started"}"#,
        );
        assert!(verify(
            "s3cr3t",
            "1700000000",
            br#"{"event_type":"session_started"}"#,
            &signature
        ));
    }

    #[test]
    fn rejects_tampered_body_secret_or_timestamp() {
        let signature = sign("s3cr3t", "1700000000", b"payload");

        assert!(!verify("s3cr3t", "1700000000", b"tampered", &signature));
        assert!(!verify("autre", "1700000000", b"payload", &signature));
        assert!(!verify("s3cr3t", "1700000001", b"payload", &signature));
    }

    #[test]
    fn rejects_malformed_hex_signature() {
        assert!(!verify("s3cr3t", "1700000000", b"payload", "pas-de-l-hex"));
    }
}
