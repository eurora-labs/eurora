-- Merge reasoning_blocks into the content JSONB array as ContentBlock::Reasoning blocks,
-- then drop the now-redundant column and its partial index.

UPDATE messages
SET content = (
    SELECT jsonb_agg(block ORDER BY ord)
    FROM (
        SELECT 0 AS ord, jsonb_build_object(
            'type', 'reasoning',
            'reasoning', elem->>'content'
        ) AS block
        FROM jsonb_array_elements(reasoning_blocks) AS elem

        UNION ALL

        SELECT row_number() OVER () AS ord, elem AS block
        FROM jsonb_array_elements(content) AS elem
    ) sub
)
WHERE reasoning_blocks IS NOT NULL
  AND reasoning_blocks != 'null'::jsonb;

DROP INDEX IF EXISTS idx_messages_reasoning;
ALTER TABLE messages DROP COLUMN reasoning_blocks;
