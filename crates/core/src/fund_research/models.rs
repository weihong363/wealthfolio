use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FundResearchSnapshot {
    pub fund_code: String,
    pub fund_name: String,
    pub snapshot_date: NaiveDate,
    pub source: String,
    pub quote: Option<FundQuote>,
    pub nav_history: Vec<FundNavPoint>,
    pub fund_holdings: Vec<FundInternalHolding>,
    pub holding_changes: Vec<FundHoldingChange>,
    pub sector_allocations: Vec<SectorAllocation>,
    pub theme_exposures: Vec<ThemeExposure>,
    pub asset_allocations: Vec<AssetAllocation>,
    pub region_allocations: Vec<RegionAllocation>,
    pub performance: Option<FundPerformance>,
    pub risk_metrics: Option<FundRiskMetrics>,
    pub announcements: Vec<FundAnnouncement>,
    pub ai_features: FundAiFeatures,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FundQuote {
    pub nav: f64,
    pub nav_date: NaiveDate,
    pub daily_return_pct: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FundNavPoint {
    pub date: NaiveDate,
    pub nav: f64,
    pub daily_return_pct: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FundInternalHolding {
    pub fund_code: String,
    pub report_date: NaiveDate,
    pub rank: Option<u32>,
    pub asset_code: Option<String>,
    pub asset_name: String,
    pub asset_type: HoldingAssetType,
    pub market: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sector: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub industry: Option<String>,
    pub weight_pct: Option<f64>,
    pub theme_tags: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum HoldingAssetType {
    Stock,
    Bond,
    Cash,
    Fund,
    Other,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FundHoldingChange {
    pub fund_code: String,
    pub report_date: NaiveDate,
    pub asset_code: Option<String>,
    pub asset_name: String,
    pub previous_weight_pct: Option<f64>,
    pub current_weight_pct: Option<f64>,
    pub weight_change_pct: Option<f64>,
    pub previous_rank: Option<u32>,
    pub current_rank: Option<u32>,
    pub change_type: HoldingChangeType,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HoldingChangeType {
    New,
    Removed,
    Increased,
    Decreased,
    Unchanged,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SectorAllocation {
    pub fund_code: String,
    pub snapshot_date: NaiveDate,
    pub sector: String,
    pub weight_pct: Option<f64>,
    pub previous_weight_pct: Option<f64>,
    pub weight_change_pct: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThemeExposure {
    pub fund_code: String,
    pub snapshot_date: NaiveDate,
    pub theme: String,
    pub weight_pct: f64,
    pub source: ThemeExposureSource,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ThemeExposureSource {
    OfficialSector,
    HoldingMapping,
    CustomRule,
    AiInferred,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AssetAllocation {
    pub fund_code: String,
    pub snapshot_date: NaiveDate,
    pub asset_class: String,
    pub weight_pct: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RegionAllocation {
    pub fund_code: String,
    pub snapshot_date: NaiveDate,
    pub region: String,
    pub weight_pct: Option<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FundPerformance {
    pub return_1m_pct: Option<f64>,
    pub return_3m_pct: Option<f64>,
    pub return_6m_pct: Option<f64>,
    pub return_1y_pct: Option<f64>,
    pub peer_rank: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FundRiskMetrics {
    pub max_drawdown_1m_pct: Option<f64>,
    pub max_drawdown_1y_pct: Option<f64>,
    pub annualized_volatility_pct: Option<f64>,
    pub sharpe_ratio: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FundAnnouncement {
    pub fund_code: String,
    pub title: String,
    pub date: Option<NaiveDate>,
    pub url: Option<String>,
    pub category: Option<AnnouncementCategory>,
    pub importance: AnnouncementImportance,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnnouncementCategory {
    ManagerChange,
    PurchaseSuspension,
    RedemptionSuspension,
    RiskWarning,
    MajorEvent,
    Other,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnnouncementImportance {
    Low,
    Medium,
    High,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HoldingConcentration {
    pub top1_weight: Option<f64>,
    pub top5_weight: Option<f64>,
    pub top10_weight: Option<f64>,
    pub hhi: Option<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FundAiFeatures {
    pub holding_concentration: Option<HoldingConcentration>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SectorRotationSignal {
    pub theme: String,
    pub signal_date: NaiveDate,
    pub momentum_1m: Option<f64>,
    pub momentum_3m: Option<f64>,
    pub inflow_score: Option<f64>,
    pub crowding_score: Option<f64>,
    pub drawdown_risk: Option<f64>,
    pub signal_strength: RotationSignalStrength,
    pub reason: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RotationSignalStrength {
    StrongInflow,
    Inflow,
    Neutral,
    Outflow,
    StrongOutflow,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RebalanceAlert {
    pub id: String,
    pub fund_code: String,
    pub alert_date: NaiveDate,
    pub alert_type: RebalanceAlertType,
    pub severity: AlertSeverity,
    pub title: String,
    pub message: String,
    pub evidence: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StockClassificationOverride {
    pub stock_key: String,
    pub asset_code: Option<String>,
    pub asset_name: String,
    pub sector: Option<String>,
    pub industry: Option<String>,
    pub theme_tags: Vec<String>,
    pub source: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpsertStockClassificationOverride {
    pub asset_code: Option<String>,
    pub asset_name: String,
    pub sector: Option<String>,
    pub industry: Option<String>,
    pub theme_tags: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RebalanceAlertType {
    ManagerChanged,
    CoreHoldingRemoved,
    ThemeExposureDrop,
    DrawdownExceeded,
    SharpeDeteriorated,
    VolatilityIncreased,
    ScaleRapidExpansion,
    ScalePersistentShrink,
    TradingSuspension,
    MajorAnnouncement,
    CrowdingTooHigh,
    PortfolioThemeConcentration,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioFundPosition {
    pub portfolio_id: String,
    pub fund_code: String,
    pub market_value_base: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioThemeExposure {
    pub portfolio_id: String,
    pub theme: String,
    pub exposure_value_base: f64,
    pub weight_pct: f64,
    pub source_funds: Vec<String>,
}

// ── Fund Holdings Lookthrough Models ──

/// A single top holding extracted from a fund snapshot.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FundTopHolding {
    pub fund_code: String,
    pub fund_name: Option<String>,
    pub report_date: NaiveDate,
    pub rank: Option<u32>,
    pub asset_code: Option<String>,
    pub asset_name: String,
    pub asset_type: HoldingAssetType,
    pub market: Option<String>,
    pub sector: Option<String>,
    pub industry: Option<String>,
    pub weight_pct: f64,
    pub theme_tags: Vec<String>,
}

/// How much a single fund contributes to a lookthrough holding.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FundLookthroughContribution {
    pub fund_code: String,
    pub fund_name: Option<String>,
    pub fund_market_value_base: f64,
    pub fund_holding_weight_pct: f64,
    pub exposure_value_base: f64,
    pub report_date: NaiveDate,
}

/// An aggregated underlying holding after lookthrough.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioFundLookthroughHolding {
    pub asset_code: Option<String>,
    pub asset_name: String,
    pub asset_type: HoldingAssetType,
    pub market: Option<String>,
    pub sector: Option<String>,
    pub industry: Option<String>,
    pub theme_tags: Vec<String>,

    pub exposure_value_base: f64,
    pub weight_pct: f64,

    pub source_funds: Vec<FundLookthroughContribution>,
}

/// Summary of the entire portfolio fund lookthrough.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioFundLookthroughSummary {
    pub portfolio_id: String,
    pub total_fund_market_value_base: f64,
    pub holdings: Vec<PortfolioFundLookthroughHolding>,
    pub missing_funds: Vec<String>,
}
