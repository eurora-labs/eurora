-- Migration: Cleanup Functions for Expired Records
-- Created: 2026-01-22 14:35:00
-- Description: Creates cleanup functions for expired/consumed ephemeral records
--              These functions should be called periodically (e.g., via pg_cron or application)

----------------------------------------------------------------
-- Cleanup function for expired OAuth state records
-- Deletes: consumed OR expired records
----------------------------------------------------------------
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

----------------------------------------------------------------
-- Cleanup function for expired/revoked refresh tokens
-- Deletes: revoked OR expired tokens older than retention period
----------------------------------------------------------------
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

----------------------------------------------------------------
-- Cleanup function for expired/consumed login tokens
-- Deletes: consumed OR expired tokens older than retention period
----------------------------------------------------------------
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

----------------------------------------------------------------
-- Master cleanup function - calls all cleanup functions
-- Returns: JSON object with counts of deleted records per table
----------------------------------------------------------------
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

----------------------------------------------------------------
-- Documentation comments
----------------------------------------------------------------
COMMENT ON FUNCTION cleanup_oauth_state(INTERVAL) IS 
    'Deletes consumed or expired OAuth state records. Default retention: 1 hour after expiry.';

COMMENT ON FUNCTION cleanup_refresh_tokens(INTERVAL) IS 
    'Deletes revoked or expired refresh tokens. Default retention: 30 days after expiry.';

COMMENT ON FUNCTION cleanup_login_tokens(INTERVAL) IS 
    'Deletes consumed or expired login tokens. Default retention: 1 day after expiry.';

COMMENT ON FUNCTION cleanup_expired_records(INTERVAL, INTERVAL, INTERVAL) IS 
    'Master cleanup function that purges all expired ephemeral records. Returns JSON with deletion counts.';

----------------------------------------------------------------
-- Example pg_cron setup (requires pg_cron extension - run manually if available)
-- 
-- To enable, run these commands as superuser:
--   CREATE EXTENSION IF NOT EXISTS pg_cron;
--   
-- Then schedule daily cleanup at 3 AM:
--   SELECT cron.schedule('daily-cleanup', '0 3 * * *', 
--       $$SELECT cleanup_expired_records()$$);
--
-- To check scheduled jobs:
--   SELECT * FROM cron.job;
--
-- To remove scheduled job:
--   SELECT cron.unschedule('daily-cleanup');
----------------------------------------------------------------
