-- Restores one row per (piece, number). Hidden rows that share a number with
-- another row (replaced versions and pending replacements) are deleted with
-- their file records; the remaining replaced or retired rows become pending,
-- the old schema's hidden state. Their S3 objects are left in place.
DROP INDEX IF EXISTS versions_piece_ready_number_key;
ALTER TABLE versions DROP COLUMN IF EXISTS replaces_version_id;

DELETE FROM versions v
WHERE v.status <> 'ready'
  AND EXISTS (
      SELECT 1 FROM versions other
      WHERE other.piece_id = v.piece_id AND other.number = v.number AND other.id <> v.id
        AND (other.status = 'ready' OR other.id < v.id)
  );
UPDATE versions SET status = 'pending' WHERE status IN ('replaced', 'retired');

ALTER TABLE versions DROP CONSTRAINT versions_status_check;
ALTER TABLE versions ADD CONSTRAINT versions_status_check
    CHECK (status IN ('pending', 'ready'));
ALTER TABLE versions ADD CONSTRAINT versions_piece_id_number_key UNIQUE (piece_id, number);
