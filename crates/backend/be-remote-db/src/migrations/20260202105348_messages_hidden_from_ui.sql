ALTER TABLE messages
    ADD COLUMN hidden_from_ui BOOLEAN NOT NULL DEFAULT false;

CREATE INDEX idx_messages_visible ON messages(thread_id)
    WHERE hidden_from_ui = false;
