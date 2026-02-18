CREATE TABLE oauth_state (
    id              UUID PRIMARY KEY,
    state           VARCHAR(128) NOT NULL,
    -- Encrypted at the application layer before storage
    pkce_verifier   BYTEA NOT NULL,
    redirect_uri    TEXT NOT NULL,
    ip_address      INET,
    consumed        BOOLEAN NOT NULL DEFAULT false,
    created_at      TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    updated_at      TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    expires_at      TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE UNIQUE INDEX idx_oauth_state_state ON oauth_state(state);
CREATE INDEX idx_oauth_state_expires_at ON oauth_state(expires_at);

CREATE TRIGGER update_oauth_state_updated_at
    BEFORE UPDATE ON oauth_state
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
