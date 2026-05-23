CREATE TABLE ingest_runs (
    id uuid PRIMARY KEY,
    source text NOT NULL,
    collection_name text NOT NULL,
    started_at timestamptz NOT NULL DEFAULT now(),
    finished_at timestamptz
);

CREATE TABLE assets (
    id uuid PRIMARY KEY,
    sha256 text NOT NULL UNIQUE,
    original_key text NOT NULL,
    preview_key text NOT NULL,
    metadata_key text NOT NULL,
    mime_type text NOT NULL,
    byte_size bigint NOT NULL,
    captured_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE collections (
    id uuid PRIMARY KEY,
    name text NOT NULL UNIQUE,
    source text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE collection_assets (
    collection_id uuid NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    asset_id uuid NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    PRIMARY KEY (collection_id, asset_id)
);

CREATE TABLE restore_requests (
    id uuid PRIMARY KEY,
    collection_id uuid REFERENCES collections(id) ON DELETE SET NULL,
    start_date date,
    end_date date,
    restore_days integer NOT NULL DEFAULT 30,
    state text NOT NULL,
    requested_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE restore_items (
    restore_request_id uuid NOT NULL REFERENCES restore_requests(id) ON DELETE CASCADE,
    asset_id uuid NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    state text NOT NULL,
    PRIMARY KEY (restore_request_id, asset_id)
);
