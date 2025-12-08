-- Migration: Login Tokens
-- Created: 2025-06-07 13:02:20
-- Description: Creates login_tokens table for desktop login token management

----------------------------------------------------------------
-- Enable UUID extension if not already enabled
----------------------------------------------------------------
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

----------------------------------------------------------------
-- Create login_tokens table
----------------------------------------------------------------
CREATE TABLE login_tokens (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    token VARCHAR(128) NOT NULL,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    user_id UUID NOT NULL,
    consumed BOOLEAN DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    
    -- Foreign key constraint
    CONSTRAINT fk_login_tokens_user_id 
        FOREIGN KEY (user_id) 
        REFERENCES users(id) 
        ON DELETE CASCADE
);

----------------------------------------------------------------
-- Create indexes for performance
----------------------------------------------------------------
CREATE UNIQUE INDEX idx_login_tokens_token ON login_tokens(token);
CREATE INDEX idx_login_tokens_user_id ON login_tokens(user_id);
CREATE INDEX idx_login_tokens_expires_at ON login_tokens(expires_at);
CREATE INDEX idx_login_tokens_active ON login_tokens(token) WHERE consumed = false;

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
COMMENT ON COLUMN login_tokens.token IS 'Unique login token string';
COMMENT ON COLUMN login_tokens.expires_at IS 'When the token expires';
COMMENT ON COLUMN login_tokens.user_id IS 'Foreign key to users table';
COMMENT ON COLUMN login_tokens.consumed IS 'Whether the token has been used for login';
COMMENT ON COLUMN login_tokens.created_at IS 'Timestamp when token was created';
COMMENT ON COLUMN login_tokens.updated_at IS 'Timestamp when token was last updated';