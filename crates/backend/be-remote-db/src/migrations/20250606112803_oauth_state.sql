-- Migration: OAuth State Store
-- Created: 2025-06-06
-- Note: PKCE verifier is stored encrypted. The application layer must handle
-- encryption/decryption using a server-side key before storing/retrieving.

----------------------------------------------------------------
-- Create oauth_state table
----------------------------------------------------------------
CREATE TABLE oauth_state (
    id              UUID PRIMARY KEY,
    state           VARCHAR(128) NOT NULL,
    -- PKCE verifier stored as encrypted bytes (application must encrypt before insert)
    pkce_verifier   BYTEA NOT NULL,
    redirect_uri    TEXT NOT NULL,
    ip_address      INET,
    consumed        BOOLEAN NOT NULL DEFAULT false,
    created_at      TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    updated_at      TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    expires_at      TIMESTAMP WITH TIME ZONE NOT NULL
);

----------------------------------------------------------------
-- Create indexes for performance
----------------------------------------------------------------
CREATE UNIQUE INDEX idx_oauth_state_state ON oauth_state(state);
CREATE INDEX idx_oauth_state_expires_at ON oauth_state(expires_at);

----------------------------------------------------------------
-- updated_at triggers (using consistent naming: update_<table>_updated_at)
----------------------------------------------------------------
CREATE TRIGGER update_oauth_state_updated_at
    BEFORE UPDATE ON oauth_state
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

----------------------------------------------------------------
-- Documentation comments
----------------------------------------------------------------
COMMENT ON TABLE oauth_state IS 'One-time CSRF/PKCE state records for OAuth callbacks';
COMMENT ON COLUMN oauth_state.state IS 'Base64-URL random value sent in OAuth "state"';
COMMENT ON COLUMN oauth_state.pkce_verifier IS 'Encrypted PKCE code_verifier (application encrypts before storage)';
COMMENT ON COLUMN oauth_state.redirect_uri IS 'Exact redirect_uri used when code issued';
COMMENT ON COLUMN oauth_state.ip_address IS 'Optional client address for defence in depth';
COMMENT ON COLUMN oauth_state.consumed IS 'Set true after successful validation';
