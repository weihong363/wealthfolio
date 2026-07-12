use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Macro-level capital observation snapshot.
///
/// A single flexible model stores every macro capital indicator (northbound /
/// southbound connect flows, margin balance, ETF flows, USD index, VIX, ...) so
/// that new indicators can be added without new tables. The `indicator` field
/// discriminates the metric and follows the constants in [`indicator`].
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MacroCapitalSnapshot {
    /// Metric discriminator, see [`indicator`] for the canonical values.
    pub indicator: String,
    /// Market scope: "CN" | "HK" | "US" | "GLOBAL".
    pub market: String,
    pub date: NaiveDate,
    /// Primary value. Meaning depends on `indicator` (net flow in 亿元, index
    /// level, balance in 亿元, ...). `unit` documents the interpretation.
    pub value: f64,
    /// Optional day-over-day change (absolute or percent depending on indicator).
    pub change: Option<f64>,
    /// Human-readable unit hint, e.g. "100M_CNY" | "index" | "100M_CNY_balance".
    pub unit: Option<String>,
    pub source: String,
}

/// Canonical indicator identifiers used by [`MacroCapitalSnapshot::indicator`].
pub mod indicator {
    /// Northbound (Stock Connect into A-shares) net flow.
    pub const NORTHBOUND: &str = "northbound";
    /// Southbound (Stock Connect into HK) net flow.
    pub const SOUTHBOUND: &str = "southbound";
    /// Margin financing + securities lending balance (两融余额).
    pub const MARGIN_BALANCE: &str = "margin_balance";
    /// ETF net subscription (positive) / redemption (negative).
    pub const ETF_NET_FLOW: &str = "etf_net_flow";
    /// ETF outstanding share change.
    pub const ETF_SHARES: &str = "etf_shares";
    /// US Dollar Index (DXY).
    pub const DXY: &str = "dxy";
    /// CBOE Volatility Index (VIX).
    pub const VIX: &str = "vix";
}
