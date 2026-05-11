-- no-transaction
-- Add `apple` to the `oauth_provider` enum.
--
-- `ALTER TYPE ... ADD VALUE` is forbidden inside a transaction block, so
-- this migration opts out of sqlx's default transaction wrapper via the
-- `-- no-transaction` directive on the first line. The directive only
-- works for single-statement files (sqlx sends the file as one query and
-- Postgres opens an implicit transaction for multi-statement queries),
-- so keep this file to one statement.

ALTER TYPE oauth_provider ADD VALUE 'apple';
