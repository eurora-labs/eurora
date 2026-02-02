-- Migration: Add hidden_from_ui field to messages
-- Created: 2026-02-02 09:25:00
-- Description: Adds hidden_from_ui boolean column to messages table
--              Used to hide certain messages from the UI while preserving them in the database
----------------------------------------------------------------
-- Add hidden_from_ui column to messages table
----------------------------------------------------------------
ALTER TABLE messages
    ADD COLUMN hidden_from_ui BOOLEAN NOT NULL DEFAULT false;
----------------------------------------------------------------
-- Create index for filtering visible messages
----------------------------------------------------------------
CREATE INDEX idx_messages_visible ON messages(conversation_id)
    WHERE hidden_from_ui = false;
----------------------------------------------------------------
-- Documentation comments
----------------------------------------------------------------
COMMENT ON COLUMN messages.hidden_from_ui IS 'Whether the message should be hidden from the user interface';
