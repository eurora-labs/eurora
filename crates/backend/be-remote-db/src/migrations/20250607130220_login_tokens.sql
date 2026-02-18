CREATE TABLE login_tokens (
    id UUID PRIMARY KEY,
    token_hash BYTEA NOT NULL,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    user_id UUID NOT NULL,
    consumed BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),

    CONSTRAINT fk_login_tokens_user_id
        FOREIGN KEY (user_id)
        REFERENCES users(id)
        ON DELETE CASCADE,

    CONSTRAINT login_tokens_chk_hash_len CHECK (octet_length(token_hash) = 32)
);

CREATE UNIQUE INDEX idx_login_tokens_token_hash ON login_tokens(token_hash);
CREATE INDEX idx_login_tokens_user_id ON login_tokens(user_id);
CREATE INDEX idx_login_tokens_expires_at ON login_tokens(expires_at);
CREATE INDEX idx_login_tokens_active ON login_tokens(token_hash) WHERE consumed = false;

CREATE TRIGGER update_login_tokens_updated_at
    BEFORE UPDATE ON login_tokens
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
