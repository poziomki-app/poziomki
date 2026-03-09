CREATE EXTENSION IF NOT EXISTS timescaledb;

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

SELECT create_hypertable('metrics_scalar_samples', 'ts');
SELECT create_hypertable('metrics_histogram_samples', 'ts');

CREATE INDEX idx_mss_chart_ts ON metrics_scalar_samples (chart, series, ts DESC);
CREATE INDEX idx_mhs_chart_ts ON metrics_histogram_samples (chart, series, bucket, ts DESC);

SELECT add_retention_policy('metrics_scalar_samples', INTERVAL '30 days');
SELECT add_retention_policy('metrics_histogram_samples', INTERVAL '30 days');
