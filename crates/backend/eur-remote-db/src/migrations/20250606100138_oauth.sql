-- Migration: Add OAuth support (Google, GitHub)
-- Created: 2025-06-06
-- Adds oauth_credentials and refresh_tokens tables

----------------------------------------------------------------
-- Enable UUID extension if not already enabled
----------------------------------------------------------------
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

----------------------------------------------------------------
-- oauth_credentials : stores provider identifiers + tokens
----------------------------------------------------------------
CREATE TABLE oauth_credentials (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL,
    provider VARCHAR(16) NOT NULL,
    provider_user_id VARCHAR(255) NOT NULL,
    access_token BYTEA,
    refresh_token BYTEA,
    access_token_expiry TIMESTAMP WITH TIME ZONE,
    scope TEXT,
    issued_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT now(),

    CONSTRAINT fk_oauth_credentials_user_id
        FOREIGN KEY (user_id)
        REFERENCES users(id)
        ON DELETE CASCADE,

    CONSTRAINT ck_oauth_provider CHECK (provider IN ('google', 'github'))
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
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL,
    token_hash TEXT NOT NULL,
    issued_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    revoked BOOLEAN DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT now(),

    CONSTRAINT fk_refresh_tokens_user_id
        FOREIGN KEY (user_id)
        REFERENCES users(id)
        ON DELETE CASCADE
);

----------------------------------------------------------------
-- Create indexes for performance
----------------------------------------------------------------
CREATE INDEX idx_refresh_tokens_user_id
    ON refresh_tokens(user_id);

CREATE INDEX idx_refresh_tokens_active
    ON refresh_tokens(user_id)
    WHERE revoked = false;

----------------------------------------------------------------
-- updated_at triggers
----------------------------------------------------------------
CREATE TRIGGER trg_oauth_credentials_updated
    BEFORE UPDATE ON oauth_credentials
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER trg_refresh_tokens_updated
    BEFORE UPDATE ON refresh_tokens
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

----------------------------------------------------------------
-- Documentation comments
----------------------------------------------------------------
COMMENT ON TABLE oauth_credentials IS 'External OAuth account bindings and provider tokens';
COMMENT ON COLUMN oauth_credentials.provider IS 'OAuth provider name (google|github)';
COMMENT ON COLUMN oauth_credentials.provider_user_id IS 'Sub/UID received from provider';
COMMENT ON COLUMN oauth_credentials.access_token IS 'Encrypted Google/GitHub access token';
COMMENT ON COLUMN oauth_credentials.refresh_token IS 'Encrypted provider refresh token';

COMMENT ON TABLE refresh_tokens IS 'Backend-issued refresh tokens for session management';
COMMENT ON COLUMN refresh_tokens.token_hash IS 'SHA-256 of refresh token + server secret';
COMMENT ON COLUMN refresh_tokens.revoked IS 'True if token manually or automatically revoked';
