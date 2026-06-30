use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioThemeExposure {
    pub portfolio_id: String,
    pub theme: String,
    pub weight_pct: f64,
    pub market_value: f64,
    pub source: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FundThemeExposureInput {
    pub fund_code: String,
    pub fund_weight_pct: f64,
    pub fund_market_value: f64,
    pub theme: String,
    pub theme_weight_pct: f64,
    pub source: String,
}
