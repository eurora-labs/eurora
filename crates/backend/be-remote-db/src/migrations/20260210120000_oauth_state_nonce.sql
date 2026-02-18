-- Encrypted at the application layer (same as pkce_verifier)
ALTER TABLE oauth_state ADD COLUMN nonce BYTEA;
