-- Replace the flat `activities` table with a parent/child model.
--
-- Before: one row per focus run, keyed by random UUID. Threads linked to
-- that single ephemeral row through `activity_threads`, so the same chat
-- could never surface "all my YouTube chats" across multiple visits.
--
-- After:
-- * `activities` is the stable parent, uniquely keyed by
--   `(user_id, identity_key)` — `identity_key` is a normalized process
--   name for the default strategy (`code`, `chrome`) and a base domain
--   label for the browser strategy (`youtube`, `github`). `youtube.com`,
--   `m.youtube.com`, and `youtube.co.uk` therefore bucket together.
-- * `activity_sessions` is the time-windowed child — one row per focus
--   run, FK-linked to its parent.
-- * `activity_threads` keeps its (activity_id, thread_id) shape but
--   `activity_id` now points at the parent, so the existing join in
--   `list_threads_for_activity` naturally aggregates every chat from any
--   session of that parent.
--
-- Per project direction, prior data is dropped wholesale — no backfill.

DROP TABLE IF EXISTS activity_threads CASCADE;
DROP TABLE IF EXISTS activity_assets CASCADE;
DROP TABLE IF EXISTS activities CASCADE;

CREATE TABLE activities (
    id              UUID PRIMARY KEY,
    user_id         UUID NOT NULL,
    identity_key    VARCHAR(500) NOT NULL,
    display_name    VARCHAR(500) NOT NULL,
    icon_asset_id   UUID,
    last_used_at    TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    created_at      TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    updated_at      TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),

    CONSTRAINT fk_activities_user_id
        FOREIGN KEY (user_id)
        REFERENCES users(id)
        ON DELETE CASCADE,

    CONSTRAINT fk_activities_icon_asset_id
        FOREIGN KEY (icon_asset_id)
        REFERENCES assets(id)
        ON DELETE SET NULL,

    CONSTRAINT uq_activities_user_identity
        UNIQUE (user_id, identity_key)
);

CREATE INDEX idx_activities_user_last_used ON activities (user_id, last_used_at DESC);

CREATE TABLE activity_sessions (
    id              UUID PRIMARY KEY,
    activity_id     UUID NOT NULL,
    user_id         UUID NOT NULL,
    process_name    VARCHAR(500) NOT NULL,
    process_id      INTEGER,
    window_title    VARCHAR(500),
    url             TEXT,
    started_at      TIMESTAMP WITH TIME ZONE NOT NULL,
    -- `ended_at IS NULL` means the session is currently live. Set once
    -- on the closing PATCH (graceful shutdown or focus transition).
    -- Crash recovery is opportunistic: inserting a new session for an
    -- activity_id auto-closes any prior open sessions for the same
    -- (user, activity) pair, so a freshly-restarted client always
    -- leaves at most one live row per activity.
    ended_at        TIMESTAMP WITH TIME ZONE,
    created_at      TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    updated_at      TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),

    CONSTRAINT fk_activity_sessions_activity_id
        FOREIGN KEY (activity_id)
        REFERENCES activities(id)
        ON DELETE CASCADE,

    CONSTRAINT fk_activity_sessions_user_id
        FOREIGN KEY (user_id)
        REFERENCES users(id)
        ON DELETE CASCADE
);

CREATE INDEX idx_activity_sessions_activity_started ON activity_sessions (activity_id, started_at DESC);
CREATE INDEX idx_activity_sessions_user_started ON activity_sessions (user_id, started_at DESC);
CREATE INDEX idx_activity_sessions_live ON activity_sessions (user_id) WHERE ended_at IS NULL;

CREATE TABLE activity_threads (
    activity_id     UUID NOT NULL,
    thread_id       UUID NOT NULL,
    created_at      TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),

    PRIMARY KEY (activity_id, thread_id),

    CONSTRAINT fk_activity_threads_activity_id
        FOREIGN KEY (activity_id)
        REFERENCES activities(id)
        ON DELETE CASCADE,

    CONSTRAINT fk_activity_threads_thread_id
        FOREIGN KEY (thread_id)
        REFERENCES threads(id)
        ON DELETE CASCADE
);

CREATE INDEX idx_activity_threads_thread_id ON activity_threads (thread_id);

CREATE TRIGGER update_activities_updated_at
    BEFORE UPDATE ON activities
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_activity_sessions_updated_at
    BEFORE UPDATE ON activity_sessions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
