-- Add tree structure for conversation branching.
-- Each message points to its parent; threads track the active leaf.

ALTER TABLE messages ADD UNIQUE (id, thread_id);

ALTER TABLE messages ADD COLUMN parent_message_id UUID;
ALTER TABLE messages ADD CONSTRAINT fk_messages_parent
    FOREIGN KEY (parent_message_id, thread_id) REFERENCES messages(id, thread_id);
CREATE INDEX idx_messages_parent ON messages(parent_message_id);

ALTER TABLE threads ADD COLUMN active_leaf_id UUID;
ALTER TABLE threads ADD CONSTRAINT fk_threads_active_leaf
    FOREIGN KEY (active_leaf_id, id) REFERENCES messages(id, thread_id);

WITH ordered AS (
    SELECT id,
           LAG(id) OVER (PARTITION BY thread_id ORDER BY created_at, id) AS prev_id
    FROM messages
)
UPDATE messages m
SET parent_message_id = o.prev_id
FROM ordered o
WHERE m.id = o.id
  AND o.prev_id IS NOT NULL;

WITH last_msg AS (
    SELECT DISTINCT ON (thread_id) thread_id, id
    FROM messages
    ORDER BY thread_id, created_at DESC, id DESC
)
UPDATE threads t
SET active_leaf_id = lm.id
FROM last_msg lm
WHERE t.id = lm.thread_id;
