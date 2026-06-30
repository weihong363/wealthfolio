use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarketSnapshot {
    pub market: String,
    pub index_name: String,
    pub price: f64,
    pub change_pct: f64,
    pub turnover: Option<f64>,
    pub timestamp: DateTime<Utc>,
    pub source: String,
}
