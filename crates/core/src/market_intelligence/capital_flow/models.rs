use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CapitalFlowSnapshot {
    pub market: String,
    pub date: NaiveDate,
    pub category: String,
    pub inflow: Option<f64>,
    pub outflow: Option<f64>,
    pub net_flow: f64,
    pub source: String,
}
