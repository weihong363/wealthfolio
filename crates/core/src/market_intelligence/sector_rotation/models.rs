use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SectorRotationSnapshot {
    pub market: String,
    pub sector: String,
    pub date: NaiveDate,
    pub net_flow: Option<f64>,
    pub change_pct: Option<f64>,
    pub turnover: Option<f64>,
    pub ranking: Option<i32>,
    pub source: String,
}
