use chrono::{DateTime, Utc};
use diesel::prelude::*;

use crate::db::schema::metrics_scalar_samples;

#[derive(Queryable, Selectable)]
#[diesel(table_name = metrics_scalar_samples)]
pub struct MetricScalarSample {
    pub ts: DateTime<Utc>,
    pub instance_id: String,
    pub chart: i16,
    pub series: i16,
    pub value: f32,
}

#[derive(Insertable)]
#[diesel(table_name = metrics_scalar_samples)]
pub struct NewMetricScalarSample {
    pub ts: DateTime<Utc>,
    pub instance_id: String,
    pub chart: i16,
    pub series: i16,
    pub value: f32,
}
