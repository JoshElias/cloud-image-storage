ALTER TABLE ingest_runs
ADD COLUMN status text NOT NULL DEFAULT 'pending',
ADD COLUMN total_assets integer NOT NULL DEFAULT 0,
ADD COLUMN completed_assets integer NOT NULL DEFAULT 0,
ADD COLUMN total_bytes bigint NOT NULL DEFAULT 0,
ADD COLUMN error_message text;

CREATE TABLE ingest_run_assets (
    ingest_run_id uuid NOT NULL REFERENCES ingest_runs(id) ON DELETE CASCADE,
    asset_id uuid NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    original_uploaded_at timestamptz,
    metadata_uploaded_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (ingest_run_id, asset_id)
);
