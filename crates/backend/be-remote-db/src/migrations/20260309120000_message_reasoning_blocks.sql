ALTER TABLE messages
    ADD COLUMN reasoning_blocks JSONB DEFAULT NULL;

CREATE INDEX idx_messages_reasoning
    ON messages (thread_id, created_at)
    WHERE reasoning_blocks IS NOT NULL;
