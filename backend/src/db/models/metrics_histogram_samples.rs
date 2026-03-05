use chrono::{DateTime, Utc};
use diesel::prelude::*;

use crate::db::schema::metrics_histogram_samples;

#[derive(Queryable, Selectable)]
#[diesel(table_name = metrics_histogram_samples)]
pub struct MetricHistogramSample {
    pub ts: DateTime<Utc>,
    pub instance_id: String,
    pub chart: i16,
    pub series: i16,
    pub bucket: i16,
    pub count: i64,
}

#[derive(Insertable)]
#[diesel(table_name = metrics_histogram_samples)]
pub struct NewMetricHistogramSample {
    pub ts: DateTime<Utc>,
    pub instance_id: String,
    pub chart: i16,
    pub series: i16,
    pub bucket: i16,
    pub count: i64,
}
