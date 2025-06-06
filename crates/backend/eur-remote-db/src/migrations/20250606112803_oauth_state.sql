-- Migration: OAuth State Store
-- Created: 2025-06-06

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE oauth_state (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    state           VARCHAR(128) NOT NULL,
    pkce_verifier   VARCHAR(256) NOT NULL,
    redirect_uri    TEXT NOT NULL,
    ip_address      INET,
    consumed        BOOLEAN      DEFAULT false,
    created_at      TIMESTAMPTZ  DEFAULT now(),
    expires_at      TIMESTAMPTZ  NOT NULL
);

CREATE UNIQUE INDEX idx_oauth_state_state ON oauth_state(state);
CREATE INDEX idx_oauth_state_expires_at ON oauth_state(expires_at);

COMMENT ON TABLE oauth_state IS 'One-time CSRF/PKCE state records for OAuth callbacks';
COMMENT ON COLUMN oauth_state.state IS 'Base64-URL random value sent in OAuth “state”';
COMMENT ON COLUMN oauth_state.pkce_verifier IS 'Original PKCE code_verifier to send to /token';
COMMENT ON COLUMN oauth_state.redirect_uri IS 'Exact redirect_uri used when code issued';
COMMENT ON COLUMN oauth_state.ip_address IS 'Optional client address for defence in depth';
COMMENT ON COLUMN oauth_state.consumed IS 'Set true after successful validation';
