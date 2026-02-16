-- Add nonce column to oauth_state for OpenID Connect nonce verification.
-- The nonce is encrypted at the application layer (same as pkce_verifier).
ALTER TABLE oauth_state ADD COLUMN nonce BYTEA;
