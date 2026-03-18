CREATE INDEX idx_messages_content_trgm ON messages USING GIN (content gin_trgm_ops);
