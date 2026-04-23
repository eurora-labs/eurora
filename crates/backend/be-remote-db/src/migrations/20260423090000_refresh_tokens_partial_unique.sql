-- Make the unique constraint on refresh_tokens.token_hash apply only to
-- active (non-revoked) rows. Revoked rows are kept for audit/cleanup but
-- must not block insertion of a new token whose hash happens to collide
-- with a stale revoked row. This is the semantically correct constraint:
-- we forbid two simultaneously-valid tokens with the same hash, not two
-- tokens that ever shared a hash.
DROP INDEX IF EXISTS idx_refresh_tokens_token_hash;

CREATE UNIQUE INDEX idx_refresh_tokens_token_hash
    ON refresh_tokens(token_hash)
    WHERE revoked = false;
