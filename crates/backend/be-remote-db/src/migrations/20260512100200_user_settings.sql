-- Per-user cloud-synced settings blob.
--
-- One row per user. The blob is stored verbatim as JSONB; schema interpretation
-- lives in `settings-core` on the client side. `updated_at` is the optimistic
-- concurrency token: clients PUT with a `base_updated_at` matching what they
-- last observed, and the server returns the current row on mismatch.
CREATE TABLE user_settings (
    user_id        UUID PRIMARY KEY,
    schema_version INTEGER NOT NULL CHECK (schema_version >= 0),
    settings       JSONB NOT NULL,
    created_at     TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    updated_at     TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),

    CONSTRAINT fk_user_settings_user_id
        FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TRIGGER update_user_settings_updated_at
    BEFORE UPDATE ON user_settings
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
