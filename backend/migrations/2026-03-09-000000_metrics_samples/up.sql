CREATE TABLE metrics_scalar_samples (
    ts          TIMESTAMPTZ NOT NULL,
    instance_id TEXT NOT NULL,
    chart       SMALLINT NOT NULL,
    series      SMALLINT NOT NULL,
    value       REAL NOT NULL
);

CREATE TABLE metrics_histogram_samples (
    ts          TIMESTAMPTZ NOT NULL,
    instance_id TEXT NOT NULL,
    chart       SMALLINT NOT NULL,
    series      SMALLINT NOT NULL,
    bucket      SMALLINT NOT NULL,
    count       BIGINT NOT NULL
);

CREATE INDEX idx_mss_chart_ts ON metrics_scalar_samples (chart, series, ts DESC);
CREATE INDEX idx_mhs_chart_ts ON metrics_histogram_samples (chart, series, bucket, ts DESC);

-- TimescaleDB: convert to hypertables and add retention if the extension is available.
-- Falls back to plain tables on vanilla PostgreSQL.
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_available_extensions WHERE name = 'timescaledb') THEN
        CREATE EXTENSION IF NOT EXISTS timescaledb;
        PERFORM create_hypertable('metrics_scalar_samples', 'ts');
        PERFORM create_hypertable('metrics_histogram_samples', 'ts');
        PERFORM add_retention_policy('metrics_scalar_samples', INTERVAL '30 days');
        PERFORM add_retention_policy('metrics_histogram_samples', INTERVAL '30 days');
    END IF;
END
$$;
