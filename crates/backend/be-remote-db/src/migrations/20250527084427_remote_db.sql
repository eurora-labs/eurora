-- Migration: Remote Database Authentication Schema
-- Created: 2025-05-27 08:44:27
-- Description: Creates users and password_credentials tables for authentication

----------------------------------------------------------------
-- Enable UUID extension if not already enabled
----------------------------------------------------------------
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

----------------------------------------------------------------
-- Create users table
----------------------------------------------------------------
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    username VARCHAR(255) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    display_name VARCHAR(255),
    email_verified BOOLEAN DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT now()
);

----------------------------------------------------------------
-- Create password_credentials table
----------------------------------------------------------------
CREATE TABLE password_credentials (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    
    -- Foreign key constraint
    CONSTRAINT fk_password_credentials_user_id 
        FOREIGN KEY (user_id) 
        REFERENCES users(id) 
        ON DELETE CASCADE
);

----------------------------------------------------------------
-- Create indexes for performance
----------------------------------------------------------------
CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_password_credentials_user_id ON password_credentials(user_id);

----------------------------------------------------------------
-- Create unique constraint to ensure one-to-one relationship
----------------------------------------------------------------
CREATE UNIQUE INDEX idx_password_credentials_user_id_unique ON password_credentials(user_id);

----------------------------------------------------------------
-- Add trigger to automatically update updated_at timestamp for users table
----------------------------------------------------------------
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at := now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

----------------------------------------------------------------
-- Add triggers to automatically update updated_at timestamp
----------------------------------------------------------------
CREATE TRIGGER update_users_updated_at 
    BEFORE UPDATE ON users 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_password_credentials_updated_at 
    BEFORE UPDATE ON password_credentials 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

----------------------------------------------------------------
-- Add comments for documentation
----------------------------------------------------------------
COMMENT ON TABLE users IS 'User accounts for authentication';
COMMENT ON TABLE password_credentials IS 'Password credentials linked to user accounts';
COMMENT ON COLUMN users.id IS 'Primary key UUID for user';
COMMENT ON COLUMN users.username IS 'Unique username for login';
COMMENT ON COLUMN users.email IS 'Unique email address for user';
COMMENT ON COLUMN users.display_name IS 'Display name for user interface';
COMMENT ON COLUMN users.email_verified IS 'Whether the email address has been verified';
COMMENT ON COLUMN users.created_at IS 'Timestamp when user account was created';
COMMENT ON COLUMN users.updated_at IS 'Timestamp when user account was last updated';
COMMENT ON COLUMN password_credentials.user_id IS 'Foreign key reference to users table';
COMMENT ON COLUMN password_credentials.password_hash IS 'Hashed password for authentication';
COMMENT ON COLUMN password_credentials.updated_at IS 'Timestamp when password was last updated';