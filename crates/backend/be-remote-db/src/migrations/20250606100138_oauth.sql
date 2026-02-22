CREATE TYPE oauth_provider AS ENUM ('google', 'github');

CREATE TABLE oauth_credentials (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    provider oauth_provider NOT NULL,
    provider_user_id VARCHAR(255) NOT NULL,
    access_token BYTEA,
    refresh_token BYTEA,
    access_token_expiry TIMESTAMP WITH TIME ZONE,
    scope TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),

    CONSTRAINT fk_oauth_credentials_user_id
        FOREIGN KEY (user_id)
        REFERENCES users(id)
        ON DELETE CASCADE
);

-- One row per provider account, one row per provider per user
CREATE UNIQUE INDEX idx_oauth_credentials_provider_user
    ON oauth_credentials(provider, provider_user_id);

CREATE UNIQUE INDEX idx_oauth_credentials_provider_userid
    ON oauth_credentials(provider, user_id);

CREATE TABLE refresh_tokens (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    token_hash BYTEA NOT NULL,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    revoked BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),

    CONSTRAINT fk_refresh_tokens_user_id
        FOREIGN KEY (user_id)
        REFERENCES users(id)
        ON DELETE CASCADE,

    CONSTRAINT refresh_tokens_chk_hash_len CHECK (octet_length(token_hash) = 32)
);

CREATE INDEX idx_refresh_tokens_user_id ON refresh_tokens(user_id);
CREATE UNIQUE INDEX idx_refresh_tokens_token_hash ON refresh_tokens(token_hash);
CREATE INDEX idx_refresh_tokens_active ON refresh_tokens(user_id) WHERE revoked = false;

CREATE TRIGGER update_oauth_credentials_updated_at
    BEFORE UPDATE ON oauth_credentials
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_refresh_tokens_updated_at
    BEFORE UPDATE ON refresh_tokens
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
