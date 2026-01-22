-- Migration: Login Tokens
-- Created: 2025-06-07 13:02:20
-- Description: Creates login_tokens table for desktop login token management
-- Security Note: Tokens are hashed (SHA-256) before storage. Never store raw tokens.

----------------------------------------------------------------
-- Create login_tokens table
----------------------------------------------------------------
CREATE TABLE login_tokens (
    id UUID PRIMARY KEY,
    -- SHA-256 hash of the token (32 bytes) - never store raw tokens
    token_hash BYTEA NOT NULL,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    user_id UUID NOT NULL,
    consumed BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    
    -- Foreign key constraint
    CONSTRAINT fk_login_tokens_user_id
        FOREIGN KEY (user_id)
        REFERENCES users(id)
        ON DELETE CASCADE,
    
    -- SHA-256 produces exactly 32 bytes
    CONSTRAINT login_tokens_chk_hash_len CHECK (octet_length(token_hash) = 32)
);

----------------------------------------------------------------
-- Create indexes for performance
----------------------------------------------------------------
CREATE UNIQUE INDEX idx_login_tokens_token_hash ON login_tokens(token_hash);
CREATE INDEX idx_login_tokens_user_id ON login_tokens(user_id);
CREATE INDEX idx_login_tokens_expires_at ON login_tokens(expires_at);
CREATE INDEX idx_login_tokens_active ON login_tokens(token_hash) WHERE consumed = false;

----------------------------------------------------------------
-- updated_at triggers
----------------------------------------------------------------
CREATE TRIGGER update_login_tokens_updated_at 
    BEFORE UPDATE ON login_tokens 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

----------------------------------------------------------------
-- Documentation comments
----------------------------------------------------------------
COMMENT ON TABLE login_tokens IS 'Desktop login tokens for secure authentication flow';
COMMENT ON COLUMN login_tokens.id IS 'Primary key UUID for login token record';
COMMENT ON COLUMN login_tokens.token_hash IS 'SHA-256 hash (32 bytes) of the login token - never store raw tokens';
COMMENT ON COLUMN login_tokens.expires_at IS 'When the token expires';
COMMENT ON COLUMN login_tokens.user_id IS 'Foreign key to users table';
COMMENT ON COLUMN login_tokens.consumed IS 'Whether the token has been used for login';
COMMENT ON COLUMN login_tokens.created_at IS 'Timestamp when token was created';
COMMENT ON COLUMN login_tokens.updated_at IS 'Timestamp when token was last updated';