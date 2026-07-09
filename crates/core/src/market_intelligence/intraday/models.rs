use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarketIntelligenceIntradaySnapshot {
    pub market: String,
    pub kind: String,
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub open: Option<f64>,
    pub close: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub volume: Option<f64>,
    pub amount: Option<f64>,
    pub net_flow: Option<f64>,
    pub change_pct: Option<f64>,
    pub turnover: Option<f64>,
    pub ranking: Option<i32>,
    pub source: String,
}
