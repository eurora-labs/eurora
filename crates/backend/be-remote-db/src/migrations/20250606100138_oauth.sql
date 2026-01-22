-- Migration: Add OAuth support (Google, GitHub)
-- Created: 2025-06-06
-- Adds oauth_credentials and refresh_tokens tables

----------------------------------------------------------------
-- Create ENUM type for OAuth providers (extensible via ALTER TYPE)
----------------------------------------------------------------
CREATE TYPE oauth_provider AS ENUM ('google', 'github');

----------------------------------------------------------------
-- oauth_credentials : stores provider identifiers + tokens
----------------------------------------------------------------
CREATE TABLE oauth_credentials (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    provider oauth_provider NOT NULL,
    provider_user_id VARCHAR(255) NOT NULL,
    access_token BYTEA,
    refresh_token BYTEA,
    access_token_expiry TIMESTAMP WITH TIME ZONE,
    scope TEXT,
    issued_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),

    CONSTRAINT fk_oauth_credentials_user_id
        FOREIGN KEY (user_id)
        REFERENCES users(id)
        ON DELETE CASCADE
);

----------------------------------------------------------------
-- Uniqueness: one row per provider/user, one row per provider_account
----------------------------------------------------------------
CREATE UNIQUE INDEX idx_oauth_credentials_provider_user
    ON oauth_credentials(provider, provider_user_id);

CREATE UNIQUE INDEX idx_oauth_credentials_provider_userid
    ON oauth_credentials(provider, user_id);

----------------------------------------------------------------
-- refresh_tokens : backend-issued long-lived tokens
----------------------------------------------------------------
CREATE TABLE refresh_tokens (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    -- SHA-256 hash stored as 32 bytes (256 bits)
    token_hash BYTEA NOT NULL,
    issued_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    revoked BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),

    CONSTRAINT fk_refresh_tokens_user_id
        FOREIGN KEY (user_id)
        REFERENCES users(id)
        ON DELETE CASCADE,
    
    -- SHA-256 produces exactly 32 bytes
    CONSTRAINT refresh_tokens_chk_hash_len CHECK (octet_length(token_hash) = 32)
);

----------------------------------------------------------------
-- Create indexes for performance
----------------------------------------------------------------
CREATE INDEX idx_refresh_tokens_user_id
    ON refresh_tokens(user_id);

-- Unique index for token lookups during validation (each token hash must be unique)
CREATE UNIQUE INDEX idx_refresh_tokens_token_hash
    ON refresh_tokens(token_hash);

CREATE INDEX idx_refresh_tokens_active
    ON refresh_tokens(user_id)
    WHERE revoked = false;

----------------------------------------------------------------
-- updated_at triggers (using consistent naming: update_<table>_updated_at)
----------------------------------------------------------------
CREATE TRIGGER update_oauth_credentials_updated_at
    BEFORE UPDATE ON oauth_credentials
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_refresh_tokens_updated_at
    BEFORE UPDATE ON refresh_tokens
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

----------------------------------------------------------------
-- Documentation comments
----------------------------------------------------------------
COMMENT ON TYPE oauth_provider IS 'Enum for OAuth providers - extend via ALTER TYPE ADD VALUE';
COMMENT ON TABLE oauth_credentials IS 'External OAuth account bindings and provider tokens';
COMMENT ON COLUMN oauth_credentials.provider IS 'OAuth provider (google|github) - uses oauth_provider enum';
COMMENT ON COLUMN oauth_credentials.provider_user_id IS 'Sub/UID received from provider';
COMMENT ON COLUMN oauth_credentials.access_token IS 'Encrypted Google/GitHub access token';
COMMENT ON COLUMN oauth_credentials.refresh_token IS 'Encrypted provider refresh token';

COMMENT ON TABLE refresh_tokens IS 'Backend-issued refresh tokens for session management';
COMMENT ON COLUMN refresh_tokens.token_hash IS 'SHA-256 hash (32 bytes) of refresh token';
COMMENT ON COLUMN refresh_tokens.revoked IS 'True if token manually or automatically revoked';
