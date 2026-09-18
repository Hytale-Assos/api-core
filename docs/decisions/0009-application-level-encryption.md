# ADR-0009: Application-level encryption and keyed hashing

- Status: Accepted
- Date: 2026-09-18
- Deciders: core maintainers

## Context

The core stores module secrets: the HMAC secret used to verify request
signatures, and the API key used to identify a module. A database dump must not
expose these. At the same time, modules are looked up by API key, which requires
a searchable value.

## Decision

Encryption is done **in the application** (`src/crypto.rs`), before insertion:
the database never holds the master key (`ENCRYPTION_KEY`, base64 32 bytes).

- Columns that must stay **secret but recoverable** are encrypted with
  **AES-256-GCM**. Stored format: `base64(nonce || ciphertext || tag)`.
  Currently: `modules.hmac_secret`.
- Columns that must be **searchable but not readable** are hashed with a
  deterministic **HMAC-SHA256** keyed by a pepper derived from the master key.
  Currently: `modules.api_key_hash`.
- Non-sensitive columns are stored in clear text.

Rule of thumb: encrypt only what must stay secret *and* be recovered; hash what
must be searched; leave the rest clear.

## Consequences

- A database dump alone cannot reveal module secrets.
- The master key is critical: losing it makes encrypted data unreadable, so it
  must be backed up and never versioned.
- Changing `ENCRYPTION_KEY` invalidates stored hashes and ciphertexts.
- Encryption is non-deterministic (random nonce), so ciphertext columns cannot
  be compared or indexed directly.
