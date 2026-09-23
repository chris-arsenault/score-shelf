-- Pieces, their versions, and the files each version holds.
-- A version is visible to readers once committed (status = 'ready').

CREATE TABLE pieces (
    id uuid PRIMARY KEY,
    slug text NOT NULL UNIQUE CHECK (slug ~ '^[a-z0-9][a-z0-9_-]{0,63}$'),
    title text NOT NULL CHECK (length(title) BETWEEN 1 AND 200),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE versions (
    id uuid PRIMARY KEY,
    piece_id uuid NOT NULL REFERENCES pieces (id),
    number integer NOT NULL CHECK (number > 0),
    label text NOT NULL CHECK (length(label) BETWEEN 1 AND 200),
    notes text NOT NULL DEFAULT '' CHECK (length(notes) <= 4000),
    source text NOT NULL CHECK (source IN ('agent', 'owner')),
    source_ref text CHECK (source_ref IS NULL OR length(source_ref) <= 200),
    status text NOT NULL CHECK (status IN ('pending', 'ready')),
    created_at timestamptz NOT NULL DEFAULT now(),
    committed_at timestamptz,
    UNIQUE (piece_id, number)
);

CREATE INDEX versions_piece_ready_idx ON versions (piece_id, number DESC)
    WHERE status = 'ready';

CREATE TABLE version_files (
    id uuid PRIMARY KEY,
    version_id uuid NOT NULL REFERENCES versions (id) ON DELETE CASCADE,
    kind text NOT NULL CHECK (kind IN ('musicxml', 'midi', 'audio', 'pdf', 'other')),
    filename text NOT NULL CHECK (length(filename) BETWEEN 1 AND 128),
    content_type text NOT NULL CHECK (length(content_type) BETWEEN 3 AND 100),
    size_bytes bigint NOT NULL CHECK (size_bytes > 0),
    object_key text NOT NULL UNIQUE,
    UNIQUE (version_id, filename)
);
