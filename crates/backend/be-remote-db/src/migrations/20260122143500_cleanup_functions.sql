CREATE OR REPLACE FUNCTION cleanup_oauth_state(
    retention_period INTERVAL DEFAULT INTERVAL '1 hour'
)
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM oauth_state
    WHERE consumed = true
       OR expires_at < (now() - retention_period);

    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION cleanup_refresh_tokens(
    retention_period INTERVAL DEFAULT INTERVAL '30 days'
)
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM refresh_tokens
    WHERE revoked = true
       OR expires_at < (now() - retention_period);

    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION cleanup_login_tokens(
    retention_period INTERVAL DEFAULT INTERVAL '1 day'
)
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM login_tokens
    WHERE consumed = true
       OR expires_at < (now() - retention_period);

    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION cleanup_expired_records(
    oauth_state_retention INTERVAL DEFAULT INTERVAL '1 hour',
    refresh_tokens_retention INTERVAL DEFAULT INTERVAL '30 days',
    login_tokens_retention INTERVAL DEFAULT INTERVAL '1 day'
)
RETURNS JSONB AS $$
DECLARE
    oauth_state_deleted INTEGER;
    refresh_tokens_deleted INTEGER;
    login_tokens_deleted INTEGER;
BEGIN
    oauth_state_deleted := cleanup_oauth_state(oauth_state_retention);
    refresh_tokens_deleted := cleanup_refresh_tokens(refresh_tokens_retention);
    login_tokens_deleted := cleanup_login_tokens(login_tokens_retention);

    RETURN jsonb_build_object(
        'oauth_state', oauth_state_deleted,
        'refresh_tokens', refresh_tokens_deleted,
        'login_tokens', login_tokens_deleted,
        'total', oauth_state_deleted + refresh_tokens_deleted + login_tokens_deleted,
        'executed_at', now()
    );
END;
$$ LANGUAGE plpgsql;
