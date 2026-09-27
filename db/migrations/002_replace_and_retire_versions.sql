-- Replacing and retiring versions, so the shelf holds only meaningful revisions.
--
-- A replacement is a new version row that takes over the number of the ready
-- version it replaces. When it commits, the old row becomes 'replaced' and the
-- new one 'ready'. A retired version is hidden; a later publish may reuse its
-- number when it was the latest. Replaced and retired rows keep their files
-- and history but are never shown or downloadable.
--
-- Numbers are therefore unique among ready versions only.

ALTER TABLE versions DROP CONSTRAINT versions_piece_id_number_key;

ALTER TABLE versions DROP CONSTRAINT versions_status_check;
ALTER TABLE versions ADD CONSTRAINT versions_status_check
    CHECK (status IN ('pending', 'ready', 'replaced', 'retired'));

ALTER TABLE versions ADD COLUMN replaces_version_id uuid REFERENCES versions (id);

CREATE UNIQUE INDEX versions_piece_ready_number_key ON versions (piece_id, number)
    WHERE status = 'ready';
