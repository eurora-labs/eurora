CREATE OR REPLACE FUNCTION extract_content_text(content jsonb) RETURNS text AS $$
  SELECT coalesce(string_agg(elem->>'text', ' '), '')
  FROM jsonb_array_elements(content) AS elem
  WHERE elem->>'text' IS NOT NULL;
$$ LANGUAGE sql IMMUTABLE PARALLEL SAFE;

DROP INDEX IF EXISTS idx_messages_search_tsv;
DROP INDEX IF EXISTS idx_messages_content_trgm;
ALTER TABLE messages DROP COLUMN search_tsv;

ALTER TABLE messages
  ALTER COLUMN content TYPE JSONB
  USING jsonb_build_array(jsonb_build_object('type', 'text', 'text', content));
ALTER TABLE messages
  ADD COLUMN search_tsv tsvector
  GENERATED ALWAYS AS (
    to_tsvector('english', immutable_unaccent(extract_content_text(content)))
  ) STORED;

CREATE INDEX idx_messages_search_tsv ON messages USING GIN (search_tsv);

CREATE INDEX idx_messages_content_trgm ON messages USING GIN (extract_content_text(content) gin_trgm_ops);
