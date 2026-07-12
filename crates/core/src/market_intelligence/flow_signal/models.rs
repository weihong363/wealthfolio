use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::super::{
    macro_capital::MacroCapitalSnapshot, market_overview::MarketSnapshot,
    sector_rotation::SectorRotationSnapshot, theme_rotation::ThemeRotationSnapshot,
};

/// Deterministic market money-flow signal derived from already-collected
/// objective data. This is rule-based only — it never uses AI/LLM inference and
/// never emits trade advice. The frontend only renders it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FlowSignal {
    /// One of the [`flow_state`] identifiers.
    pub state: String,
    /// Confidence in the assessment, 0.0..=1.0. Driven by input coverage and
    /// how strongly the rule conditions are met. `unknown` is always 0.0.
    pub confidence: f64,
    /// Human-readable evidence lines explaining the decision.
    pub evidence: Vec<String>,
    /// When the signal was computed.
    pub timestamp: DateTime<Utc>,
    /// Distinct data sources that contributed to the signal.
    pub sources: Vec<String>,
    /// Whether every key input (breadth, turnover, connect flow) was present.
    pub data_complete: bool,
    /// Supplementary market-liquidity gauges surfaced alongside the signal.
    pub liquidity: MarketLiquidityMetrics,
}

impl FlowSignal {
    /// Placeholder signal used before the real assessment is computed and when
    /// there is not enough data to judge. Never fabricates a state.
    pub fn unknown() -> Self {
        Self {
            state: flow_state::UNKNOWN.to_string(),
            confidence: 0.0,
            evidence: Vec::new(),
            timestamp: Utc::now(),
            sources: Vec::new(),
            data_complete: false,
            liquidity: MarketLiquidityMetrics::default(),
        }
    }
}

/// Supplementary market-liquidity indicators. Every field is optional so a
/// missing upstream degrades to "unknown" rather than a fabricated value.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarketLiquidityMetrics {
    /// Combined turnover of the Shanghai + Shenzhen benchmark indices.
    pub total_turnover: Option<f64>,
    /// Day-over-day turnover change in percent.
    pub turnover_change_pct: Option<f64>,
    /// Number of advancing sectors on the latest date (sector-level breadth
    /// proxy; individual-stock advance/decline counts are not collected).
    pub advancing: Option<i32>,
    /// Number of declining sectors on the latest date.
    pub declining: Option<i32>,
    /// Advancing / declining ratio.
    pub advance_decline_ratio: Option<f64>,
    /// Freshest date backing these liquidity metrics.
    pub as_of: Option<NaiveDate>,
}

/// Canonical flow-signal state identifiers. Values intentionally match the
/// existing market-regime states so the frontend can reuse styling.
pub mod flow_state {
    /// Liquidity improving and most sectors flowing in together.
    pub const NEW_MONEY: &str = "new_money";
    /// Aggregate money roughly flat but clear sector inflow/outflow substitution.
    pub const ROTATION: &str = "rotation";
    /// Macro liquidity falling and most sectors flowing out.
    pub const OUTFLOW: &str = "outflow";
    /// Data present but no decisive direction.
    pub const NEUTRAL: &str = "neutral";
    /// Not enough data to judge.
    pub const UNKNOWN: &str = "unknown";
}

/// Objective inputs for [`MarketFlowSignalService::evaluate`]. All borrowed from
/// the already-assembled summary; the service performs no data collection.
pub struct FlowSignalInputs<'a> {
    pub sector_rotation: &'a [SectorRotationSnapshot],
    pub theme_rotation: &'a [ThemeRotationSnapshot],
    pub macro_capital: &'a [MacroCapitalSnapshot],
    pub market_overview: &'a [MarketSnapshot],
    /// Previous trading day's combined market turnover, if resolvable from
    /// historical snapshots. Used to compute the turnover change rate.
    pub previous_turnover: Option<f64>,
}
