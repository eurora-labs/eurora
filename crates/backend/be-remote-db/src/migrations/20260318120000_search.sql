CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS unaccent;

CREATE OR REPLACE FUNCTION immutable_unaccent(text) RETURNS text AS $$
  SELECT unaccent($1);
$$ LANGUAGE sql IMMUTABLE PARALLEL SAFE;

ALTER TABLE messages
  ALTER COLUMN content TYPE TEXT
  USING CASE
    WHEN jsonb_typeof(content) = 'string' THEN content #>> '{}'
    ELSE content::text
  END;

ALTER TABLE messages
  ADD COLUMN search_tsv tsvector
  GENERATED ALWAYS AS (
    to_tsvector('english', immutable_unaccent(coalesce(content, '')))
  ) STORED;

CREATE INDEX idx_messages_search_tsv ON messages USING GIN (search_tsv);

CREATE INDEX idx_threads_title_trgm ON threads USING GIN (title gin_trgm_ops);

CREATE INDEX idx_threads_title_tsv ON threads
  USING GIN (to_tsvector('english', immutable_unaccent(coalesce(title, ''))));
