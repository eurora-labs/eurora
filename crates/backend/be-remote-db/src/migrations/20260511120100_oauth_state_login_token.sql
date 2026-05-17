-- Capture the desktop-pairing `login_token` at OAuth URL issue time
-- (instead of threading it through the `LoginRequest::ThirdParty` body
-- on callback). Apple Sign In form-posts directly to the backend — the
-- SPA never sees `code`/`state` — so the old client-mediated path
-- doesn't fit. Moving every provider onto an at-issue-time column
-- removes the dual-mechanism debt.
--
-- Encrypted at the application layer (same convention as
-- `pkce_verifier` and `nonce`).
ALTER TABLE oauth_state ADD COLUMN login_token BYTEA;
