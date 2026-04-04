DROP INDEX IF EXISTS idx_messages_visible;
ALTER TABLE messages DROP COLUMN IF EXISTS hidden_from_ui;
