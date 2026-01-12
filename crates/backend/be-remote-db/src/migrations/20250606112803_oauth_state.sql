-- Migration: OAuth State Store
-- Created: 2025-06-06

----------------------------------------------------------------
-- Enable UUID extension if not already enabled
----------------------------------------------------------------
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

----------------------------------------------------------------
-- Create oauth_state table
----------------------------------------------------------------
CREATE TABLE oauth_state (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    state           VARCHAR(128) NOT NULL,
    pkce_verifier   VARCHAR(256) NOT NULL,
    redirect_uri    TEXT NOT NULL,
    ip_address      INET,
    consumed        BOOLEAN      DEFAULT false,
    created_at      TIMESTAMPTZ  DEFAULT now(),
    updated_at      TIMESTAMPTZ  DEFAULT now(),
    expires_at      TIMESTAMPTZ  NOT NULL
);

----------------------------------------------------------------
-- Create indexes for performance
----------------------------------------------------------------
CREATE UNIQUE INDEX idx_oauth_state_state ON oauth_state(state);
CREATE INDEX idx_oauth_state_expires_at ON oauth_state(expires_at);

----------------------------------------------------------------
-- updated_at triggers
----------------------------------------------------------------
CREATE TRIGGER trg_oauth_state_updated
    BEFORE UPDATE ON oauth_state
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

----------------------------------------------------------------
-- Documentation comments
----------------------------------------------------------------
COMMENT ON TABLE oauth_state IS 'One-time CSRF/PKCE state records for OAuth callbacks';
COMMENT ON COLUMN oauth_state.state IS 'Base64-URL random value sent in OAuth “state”';
COMMENT ON COLUMN oauth_state.pkce_verifier IS 'Original PKCE code_verifier to send to /token';
COMMENT ON COLUMN oauth_state.redirect_uri IS 'Exact redirect_uri used when code issued';
COMMENT ON COLUMN oauth_state.ip_address IS 'Optional client address for defence in depth';
COMMENT ON COLUMN oauth_state.consumed IS 'Set true after successful validation';
