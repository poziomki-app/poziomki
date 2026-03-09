use chrono::{DateTime, Utc};
use diesel::prelude::*;

use crate::db::schema::{metrics_histogram_samples, metrics_scalar_samples};

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = metrics_scalar_samples)]
pub struct ScalarSample {
    pub ts: DateTime<Utc>,
    pub instance_id: String,
    pub chart: i16,
    pub series: i16,
    pub value: f32,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = metrics_scalar_samples)]
pub struct NewScalarSample {
    pub ts: DateTime<Utc>,
    pub instance_id: String,
    pub chart: i16,
    pub series: i16,
    pub value: f32,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = metrics_histogram_samples)]
pub struct HistogramSample {
    pub ts: DateTime<Utc>,
    pub instance_id: String,
    pub chart: i16,
    pub series: i16,
    pub bucket: i16,
    pub count: i64,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = metrics_histogram_samples)]
pub struct NewHistogramSample {
    pub ts: DateTime<Utc>,
    pub instance_id: String,
    pub chart: i16,
    pub series: i16,
    pub bucket: i16,
    pub count: i64,
}
