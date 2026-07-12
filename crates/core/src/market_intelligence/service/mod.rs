use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use chrono::{Duration, NaiveDate, Utc};
use log::warn;
use rust_decimal::prelude::ToPrimitive;

use super::{
    capital_flow::{
        CapitalFlowProvider, CapitalFlowRepository, CapitalFlowService, CapitalFlowSnapshot,
    },
    eastmoney::{default_theme_mappings, EastmoneyMarketIntelligenceProvider},
    flow_signal::{flow_state, FlowSignal, FlowSignalInputs, MarketFlowSignalService},
    intraday::{
        IntradayMarketIntelligenceProvider, MarketIntelligenceIntradayRepository,
        MarketIntelligenceIntradayService,
    },
    macro_capital::{
        models::indicator, MacroCapitalProvider, MacroCapitalRepository, MacroCapitalService,
        MacroCapitalSnapshot,
    },
    market_overview::{MarketOverviewService, MarketSnapshot, MarketSnapshotRepository},
    portfolio_exposure::{
        PortfolioExposureService, PortfolioThemeExposure, PortfolioThemeExposureRepository,
    },
    sector_rotation::{
        SectorRotationProvider, SectorRotationRepository, SectorRotationService,
        SectorRotationSnapshot,
    },
    theme_rotation::{
        ThemeMapping, ThemeRotationRepository, ThemeRotationService, ThemeRotationSnapshot,
    },
};
use crate::{
    errors::Result,
    fund_research::FundResearchService,
    quotes::{constants::DATA_SOURCE_YAHOO, QuoteServiceTrait},
};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarketIntelligenceSummary {
    pub market_overview: Vec<MarketSnapshot>,
    pub capital_flow: Vec<CapitalFlowSnapshot>,
    pub sector_rotation: Vec<SectorRotationSnapshot>,
    pub theme_rotation: Vec<ThemeRotationSnapshot>,
    pub portfolio_exposure: Vec<PortfolioThemeExposure>,
    pub macro_capital: Vec<MacroCapitalSnapshot>,
    pub market_regime: MarketRegimeAssessment,
    pub flow_signal: FlowSignal,
    pub trends: MarketIntelligenceTrends,
    pub data_status: Vec<MarketIntelligenceDataStatus>,
}

/// Deterministic (non-AI) assessment of the current market money regime,
/// derived entirely from already-collected objective data. The frontend only
/// renders this; all judgement happens here in the service layer.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarketRegimeAssessment {
    /// One of the [`regime`] state identifiers.
    pub state: String,
    /// Human-readable evidence lines explaining the decision.
    pub rationale: Vec<String>,
    /// Date of the freshest data used in the assessment.
    pub as_of: Option<NaiveDate>,
    /// Distinct data sources that contributed to the assessment.
    pub sources: Vec<String>,
    /// Whether every key input (breadth, main flow, connect flow) was present.
    pub data_complete: bool,
    pub metrics: MarketRegimeMetrics,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarketRegimeMetrics {
    /// Share of sectors with positive net flow on the latest date (0..1).
    pub breadth: Option<f64>,
    /// Net / gross sector flow ratio on the latest date (-1..1).
    pub net_ratio: Option<f64>,
    pub inflow_sectors: i32,
    pub outflow_sectors: i32,
    pub main_net_flow: Option<f64>,
    pub northbound_net_flow: Option<f64>,
    pub margin_balance: Option<f64>,
}

/// Canonical market regime state identifiers.
pub mod regime {
    pub const NEW_MONEY: &str = "new_money";
    pub const ROTATION: &str = "rotation";
    pub const OUTFLOW: &str = "outflow";
    pub const NEUTRAL: &str = "neutral";
    pub const UNKNOWN: &str = "unknown";
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarketIntelligenceTrends {
    pub granularity: String,
    pub windows: Vec<MarketIntelligenceTrendWindow>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarketIntelligenceTrendWindow {
    pub window: String,
    pub days: Option<i64>,
    pub capital_flow: FlowTrend,
    pub sector_rotation: FlowTrend,
    pub theme_rotation: ThemeTrend,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FlowTrend {
    pub series: Vec<NamedTimeSeries>,
    pub top_inflows: Vec<RankingItem>,
    pub top_outflows: Vec<RankingItem>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThemeTrend {
    pub flow_score_series: Vec<NamedTimeSeries>,
    pub momentum_series: Vec<NamedTimeSeries>,
    pub top_themes: Vec<RankingItem>,
    pub bottom_themes: Vec<RankingItem>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NamedTimeSeries {
    pub name: String,
    pub market: Option<String>,
    pub points: Vec<TimeSeriesPoint>,
    pub latest_value: Option<f64>,
    pub cumulative_value: f64,
    pub source: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TimeSeriesPoint {
    pub date: NaiveDate,
    pub value: f64,
    pub cumulative: f64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RankingItem {
    pub name: String,
    pub market: Option<String>,
    pub value: f64,
    pub secondary_value: Option<f64>,
    pub ranking: Option<i32>,
    pub date: Option<NaiveDate>,
    pub source: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarketIntelligenceDataStatus {
    pub section: String,
    pub available: bool,
    pub message: Option<String>,
    pub source: Option<String>,
}

pub struct MarketIntelligenceService {
    pub market_overview: MarketOverviewService,
    pub capital_flow: CapitalFlowService,
    pub sector_rotation: SectorRotationService,
    pub theme_rotation: ThemeRotationService,
    pub intraday: MarketIntelligenceIntradayService,
    pub portfolio_exposure: PortfolioExposureService,
    pub macro_capital: MacroCapitalService,
    quote_service: Option<Arc<dyn QuoteServiceTrait>>,
    fund_research_service: Option<Arc<FundResearchService>>,
}

impl MarketIntelligenceService {
    pub fn new(
        market_overview_repository: Arc<dyn MarketSnapshotRepository>,
        capital_flow_repository: Arc<dyn CapitalFlowRepository>,
        sector_rotation_repository: Arc<dyn SectorRotationRepository>,
        theme_rotation_repository: Arc<dyn ThemeRotationRepository>,
        intraday_repository: Arc<dyn MarketIntelligenceIntradayRepository>,
        portfolio_exposure_repository: Arc<dyn PortfolioThemeExposureRepository>,
        macro_capital_repository: Arc<dyn MacroCapitalRepository>,
    ) -> Self {
        Self {
            market_overview: MarketOverviewService::new(market_overview_repository),
            capital_flow: CapitalFlowService::new(capital_flow_repository),
            sector_rotation: SectorRotationService::new(sector_rotation_repository),
            theme_rotation: ThemeRotationService::new(theme_rotation_repository),
            intraday: MarketIntelligenceIntradayService::new(intraday_repository),
            portfolio_exposure: PortfolioExposureService::new(portfolio_exposure_repository),
            macro_capital: MacroCapitalService::new(macro_capital_repository),
            quote_service: None,
            fund_research_service: None,
        }
    }

    pub fn with_quote_service(mut self, quote_service: Arc<dyn QuoteServiceTrait>) -> Self {
        self.quote_service = Some(quote_service);
        self
    }

    pub fn with_fund_research_service(
        mut self,
        fund_research_service: Arc<FundResearchService>,
    ) -> Self {
        self.fund_research_service = Some(fund_research_service);
        self
    }

    pub async fn summary(&self, portfolio_id: Option<&str>) -> Result<MarketIntelligenceSummary> {
        self.summary_for_window(portfolio_id, None).await
    }

    pub async fn summary_for_window(
        &self,
        portfolio_id: Option<&str>,
        _window: Option<&str>,
    ) -> Result<MarketIntelligenceSummary> {
        let portfolio_exposure = match portfolio_id {
            Some(portfolio_id) => {
                self.latest_or_calculate_portfolio_exposure(portfolio_id)
                    .await?
            }
            None => Vec::new(),
        };
        let summary = MarketIntelligenceSummary {
            market_overview: self.market_overview.latest(None).await?,
            capital_flow: self.capital_flow.snapshots(None, None, None, 500).await?,
            sector_rotation: self
                .sector_rotation
                .snapshots(None, None, None, 500)
                .await?,
            theme_rotation: self.theme_rotation.snapshots(None, None, 500).await?,
            portfolio_exposure,
            macro_capital: self.macro_capital.snapshots(None, None, None, 500).await?,
            market_regime: MarketRegimeAssessment::default(),
            flow_signal: FlowSignal::unknown(),
            trends: self.trends().await?,
            data_status: Vec::new(),
        };

        if summary.has_market_data() {
            return Ok(self.finalize(summary).await);
        }

        match self.refresh_market_data().await {
            Ok(mut refreshed) => {
                if let Some(portfolio_id) = portfolio_id {
                    refreshed.portfolio_exposure = self
                        .latest_or_calculate_portfolio_exposure(portfolio_id)
                        .await?;
                }
                Ok(self.finalize(refreshed).await)
            }
            Err(error) => {
                warn!("Failed to auto-refresh market intelligence summary: {error}");
                Ok(self.finalize(summary).await)
            }
        }
    }

    pub async fn refresh_market_data(&self) -> Result<MarketIntelligenceSummary> {
        let market_overview = self.fetch_benchmark_market_snapshots().await?;

        if !market_overview.is_empty() {
            self.market_overview
                .ingest_snapshots(market_overview.clone())
                .await?;
        }

        let provider = EastmoneyMarketIntelligenceProvider::default();
        self.refresh_intraday_market_data_with_provider(&provider)
            .await?;

        let capital_flow = match provider.fetch_capital_flow_snapshots(None).await {
            Ok(snapshots) => {
                self.capital_flow
                    .ingest_snapshots(snapshots.clone())
                    .await?;
                snapshots
            }
            Err(error) => {
                warn!("Failed to fetch Eastmoney capital flow: {error}");
                Vec::new()
            }
        };

        let sector_rotation = match provider.fetch_sector_rotation_snapshots(None).await {
            Ok(snapshots) => {
                self.sector_rotation
                    .ingest_snapshots(snapshots.clone())
                    .await?;
                snapshots
            }
            Err(error) => {
                warn!("Failed to fetch Eastmoney sector rotation: {error}");
                Vec::new()
            }
        };

        let theme_rotation = if sector_rotation.is_empty() {
            Vec::new()
        } else {
            self.theme_rotation
                .calculate_and_save(theme_mappings_for(&sector_rotation), &sector_rotation)
                .await?
        };

        let macro_capital = self.refresh_macro_capital_with_provider(&provider).await;

        Ok(self
            .finalize(MarketIntelligenceSummary {
                market_overview,
                capital_flow,
                sector_rotation,
                theme_rotation,
                portfolio_exposure: Vec::new(),
                macro_capital,
                market_regime: MarketRegimeAssessment::default(),
                flow_signal: FlowSignal::unknown(),
                trends: self.trends().await?,
                data_status: Vec::new(),
            })
            .await)
    }

    pub async fn refresh_intraday_market_data(&self) -> Result<usize> {
        let provider = EastmoneyMarketIntelligenceProvider::default();
        self.refresh_intraday_market_data_with_provider(&provider)
            .await
    }

    pub async fn ingest_snapshots(
        &self,
        market_overview: Vec<MarketSnapshot>,
        capital_flow: Vec<CapitalFlowSnapshot>,
        sector_rotation: Vec<SectorRotationSnapshot>,
    ) -> Result<MarketIntelligenceSummary> {
        if !market_overview.is_empty() {
            self.market_overview
                .ingest_snapshots(market_overview.clone())
                .await?;
        }
        if !capital_flow.is_empty() {
            self.capital_flow
                .ingest_snapshots(capital_flow.clone())
                .await?;
        }
        if !sector_rotation.is_empty() {
            self.sector_rotation
                .ingest_snapshots(sector_rotation.clone())
                .await?;
        }

        let theme_rotation = if sector_rotation.is_empty() {
            Vec::new()
        } else {
            self.theme_rotation
                .calculate_and_save(theme_mappings_for(&sector_rotation), &sector_rotation)
                .await?
        };

        Ok(self
            .finalize(MarketIntelligenceSummary {
                market_overview,
                capital_flow,
                sector_rotation,
                theme_rotation,
                portfolio_exposure: Vec::new(),
                macro_capital: self.macro_capital.snapshots(None, None, None, 500).await?,
                market_regime: MarketRegimeAssessment::default(),
                flow_signal: FlowSignal::unknown(),
                trends: self.trends().await?,
                data_status: Vec::new(),
            })
            .await)
    }

    async fn trends(&self) -> Result<MarketIntelligenceTrends> {
        let since = Some(Utc::now().date_naive() - Duration::days(365));
        let capital_flow = self
            .capital_flow
            .snapshots(None, None, since, TREND_SNAPSHOT_LIMIT)
            .await?;
        let sector_rotation = self
            .sector_rotation
            .snapshots(None, None, since, TREND_SNAPSHOT_LIMIT)
            .await?;
        let mut theme_rotation = self
            .theme_rotation
            .snapshots(None, since, TREND_SNAPSHOT_LIMIT)
            .await?;
        if !sector_rotation.is_empty() {
            let derived_theme_rotation = derive_theme_rotation_history(&sector_rotation);
            let missing_theme_rotation =
                missing_theme_rotation_snapshots(&theme_rotation, &derived_theme_rotation);
            if !missing_theme_rotation.is_empty() {
                self.theme_rotation
                    .ingest_snapshots(missing_theme_rotation.clone())
                    .await?;
                theme_rotation.extend(missing_theme_rotation);
            }
        }

        Ok(build_trends(
            &capital_flow,
            &sector_rotation,
            &theme_rotation,
        ))
    }

    async fn refresh_intraday_market_data_with_provider(
        &self,
        provider: &EastmoneyMarketIntelligenceProvider,
    ) -> Result<usize> {
        match provider.fetch_intraday_snapshots(None).await {
            Ok(snapshots) => {
                let count = snapshots.len();
                if !snapshots.is_empty() {
                    self.intraday.ingest_snapshots(snapshots).await?;
                }
                Ok(count)
            }
            Err(error) => {
                warn!("Failed to fetch Eastmoney intraday market intelligence: {error}");
                Ok(0)
            }
        }
    }

    async fn latest_or_calculate_portfolio_exposure(
        &self,
        portfolio_id: &str,
    ) -> Result<Vec<PortfolioThemeExposure>> {
        let existing = self.portfolio_exposure.latest(portfolio_id).await?;
        if !existing.is_empty() {
            return Ok(existing);
        }

        let Some(fund_research_service) = &self.fund_research_service else {
            return Ok(existing);
        };
        let fund_exposures = fund_research_service
            .analyze_portfolio_theme_exposure(portfolio_id)
            .await?;
        let now = Utc::now();
        let exposures = fund_exposures
            .into_iter()
            .map(|exposure| PortfolioThemeExposure {
                portfolio_id: exposure.portfolio_id,
                theme: exposure.theme,
                weight_pct: exposure.weight_pct,
                market_value: exposure.exposure_value_base,
                source: if exposure.source_funds.is_empty() {
                    "fund_research_lookthrough".to_string()
                } else {
                    format!(
                        "fund_research_lookthrough:{}",
                        exposure.source_funds.join(",")
                    )
                },
                timestamp: now,
            })
            .collect::<Vec<_>>();

        self.portfolio_exposure.save_exposures(&exposures).await?;
        Ok(exposures)
    }

    async fn fetch_benchmark_market_snapshots(&self) -> Result<Vec<MarketSnapshot>> {
        let Some(quote_service) = &self.quote_service else {
            return Ok(Vec::new());
        };

        let end = Utc::now().date_naive();
        let start = end - Duration::days(10);
        let mut snapshots = Vec::new();

        for benchmark in benchmark_indices() {
            let Some((symbol, quotes)) = fetch_first_available_benchmark_quotes(
                quote_service.as_ref(),
                benchmark,
                start,
                end,
            )
            .await?
            else {
                continue;
            };

            let mut quotes = quotes;
            quotes.sort_by_key(|quote| quote.timestamp);
            let Some(latest) = quotes.last() else {
                continue;
            };
            let previous = quotes
                .iter()
                .rev()
                .skip(1)
                .find(|quote| quote.close > rust_decimal::Decimal::ZERO);
            let Some(price) = latest.close.to_f64() else {
                continue;
            };
            let change_pct = previous
                .and_then(|quote| {
                    let previous_close = quote.close.to_f64()?;
                    if previous_close <= 0.0 {
                        return None;
                    }
                    Some((price - previous_close) / previous_close * 100.0)
                })
                .unwrap_or_default();

            snapshots.push(MarketSnapshot {
                market: benchmark.market.to_string(),
                index_name: benchmark.name.to_string(),
                price,
                change_pct,
                turnover: latest.volume.to_f64(),
                timestamp: latest.timestamp,
                source: format!("yahoo_alpha_vantage:{symbol}"),
            });
        }

        Ok(snapshots)
    }

    /// Refresh macro capital indicators: Eastmoney connect/margin flows (best
    /// effort) plus DXY and VIX sourced from the shared QuoteService. Any
    /// individual failure degrades to "no data" rather than failing refresh.
    async fn refresh_macro_capital_with_provider(
        &self,
        provider: &EastmoneyMarketIntelligenceProvider,
    ) -> Vec<MacroCapitalSnapshot> {
        let mut snapshots = match provider.fetch_macro_capital_snapshots(None).await {
            Ok(snapshots) => snapshots,
            Err(error) => {
                warn!("Failed to fetch Eastmoney macro capital: {error}");
                Vec::new()
            }
        };
        snapshots.extend(self.fetch_macro_index_snapshots().await);

        if !snapshots.is_empty() {
            if let Err(error) = self.macro_capital.ingest_snapshots(snapshots.clone()).await {
                warn!("Failed to persist macro capital snapshots: {error}");
            }
        }
        snapshots
    }

    /// Fetch DXY and VIX daily levels via the shared QuoteService (Yahoo),
    /// reusing the same benchmark-quote capability as the market overview.
    async fn fetch_macro_index_snapshots(&self) -> Vec<MacroCapitalSnapshot> {
        let Some(quote_service) = &self.quote_service else {
            return Vec::new();
        };
        let end = Utc::now().date_naive();
        let start = end - Duration::days(10);
        let mut snapshots = Vec::new();

        for index in macro_indices() {
            let benchmark = BenchmarkIndex {
                market: index.market,
                name: index.indicator,
                symbol: index.symbol,
                fallback_symbols: index.fallback_symbols,
                currency: index.currency,
            };
            let quotes = match fetch_first_available_benchmark_quotes(
                quote_service.as_ref(),
                &benchmark,
                start,
                end,
            )
            .await
            {
                Ok(Some((_, quotes))) => quotes,
                Ok(None) => continue,
                Err(error) => {
                    warn!("Failed to fetch macro index {}: {error}", index.symbol);
                    continue;
                }
            };

            let mut quotes = quotes;
            quotes.sort_by_key(|quote| quote.timestamp);
            let Some(latest) = quotes.last() else {
                continue;
            };
            let Some(price) = latest.close.to_f64() else {
                continue;
            };
            let change = quotes
                .iter()
                .rev()
                .skip(1)
                .find(|quote| quote.close > rust_decimal::Decimal::ZERO)
                .and_then(|quote| {
                    let previous_close = quote.close.to_f64()?;
                    if previous_close <= 0.0 {
                        return None;
                    }
                    Some((price - previous_close) / previous_close * 100.0)
                });
            snapshots.push(MacroCapitalSnapshot {
                indicator: index.indicator.to_string(),
                market: index.market.to_string(),
                date: latest.timestamp.date_naive(),
                value: price,
                change,
                unit: Some("index".to_string()),
                source: format!("yahoo:{}", index.symbol),
            });
        }
        snapshots
    }

    /// Finalize a summary: compute the market regime (legacy field, kept for
    /// backward compatibility), the new flow signal, and per-section data
    /// status. Async because the flow signal needs the previous trading day's
    /// turnover from historical market snapshots.
    async fn finalize(&self, mut summary: MarketIntelligenceSummary) -> MarketIntelligenceSummary {
        summary.market_regime = assess_market_regime(&summary);
        let previous_turnover = self.previous_market_turnover(&summary).await;
        summary.flow_signal = MarketFlowSignalService::new().evaluate(&FlowSignalInputs {
            sector_rotation: &summary.sector_rotation,
            theme_rotation: &summary.theme_rotation,
            macro_capital: &summary.macro_capital,
            market_overview: &summary.market_overview,
            previous_turnover,
        });
        summary.data_status = data_status(&summary);
        summary
    }

    /// Resolve the previous trading day's combined CN turnover from the market
    /// snapshot history, so the flow signal can compute a turnover change rate.
    /// Returns None (never fabricates) when history is unavailable.
    async fn previous_market_turnover(&self, summary: &MarketIntelligenceSummary) -> Option<f64> {
        let latest_date = summary
            .market_overview
            .iter()
            .filter(|s| s.market == "CN")
            .map(|s| s.timestamp.date_naive())
            .max()?;

        let cn_indices: Vec<&str> = summary
            .market_overview
            .iter()
            .filter(|s| s.market == "CN" && s.turnover.is_some())
            .map(|s| s.index_name.as_str())
            .collect();
        if cn_indices.is_empty() {
            return None;
        }

        let mut previous_total = 0.0;
        let mut found_any = false;
        for index_name in cn_indices {
            let history = self
                .market_overview
                .history("CN", index_name, 10)
                .await
                .unwrap_or_default();
            let previous = history
                .iter()
                .filter(|s| s.timestamp.date_naive() < latest_date)
                .filter_map(|s| s.turnover)
                .next();
            if let Some(turnover) = previous {
                previous_total += turnover;
                found_any = true;
            }
        }
        (found_any && previous_total > 0.0).then_some(previous_total)
    }
}

impl MarketIntelligenceSummary {
    fn has_market_data(&self) -> bool {
        !self.market_overview.is_empty()
            || !self.capital_flow.is_empty()
            || !self.sector_rotation.is_empty()
            || !self.theme_rotation.is_empty()
    }
}

const TREND_SNAPSHOT_LIMIT: usize = 20_000;
const RANKING_LIMIT: usize = 10;
const SERIES_LIMIT: usize = 32;

type TrendGroupKey = (String, String);
type TrendRow = (NaiveDate, f64, Option<String>, Option<i32>);
type TrendGroups = BTreeMap<TrendGroupKey, Vec<TrendRow>>;
type RankedSeries = (
    NamedTimeSeries,
    Option<i32>,
    Option<NaiveDate>,
    Option<String>,
);

struct TrendWindowDef {
    key: &'static str,
    days: Option<i64>,
}

fn build_trends(
    capital_flow: &[CapitalFlowSnapshot],
    sector_rotation: &[SectorRotationSnapshot],
    theme_rotation: &[ThemeRotationSnapshot],
) -> MarketIntelligenceTrends {
    let windows = trend_windows()
        .iter()
        .map(|window| MarketIntelligenceTrendWindow {
            window: window.key.to_string(),
            days: window.days,
            capital_flow: build_capital_flow_trend(capital_flow, window.days),
            sector_rotation: build_sector_rotation_trend(sector_rotation, window.days),
            theme_rotation: build_theme_rotation_trend(theme_rotation, window.days),
        })
        .collect();

    MarketIntelligenceTrends {
        granularity: "daily".to_string(),
        windows,
    }
}

fn derive_theme_rotation_history(
    sector_rotation: &[SectorRotationSnapshot],
) -> Vec<ThemeRotationSnapshot> {
    let mut sectors_by_date: BTreeMap<NaiveDate, Vec<SectorRotationSnapshot>> = BTreeMap::new();
    for snapshot in sector_rotation {
        sectors_by_date
            .entry(snapshot.date)
            .or_default()
            .push(snapshot.clone());
    }

    let rules = super::theme_rotation::ThemeRotationRules::new(theme_mappings_for(sector_rotation));
    sectors_by_date
        .values()
        .flat_map(|snapshots| rules.calculate(snapshots))
        .collect()
}

fn missing_theme_rotation_snapshots(
    existing: &[ThemeRotationSnapshot],
    derived: &[ThemeRotationSnapshot],
) -> Vec<ThemeRotationSnapshot> {
    let existing_keys = existing
        .iter()
        .map(|snapshot| (snapshot.theme.to_lowercase(), snapshot.date))
        .collect::<BTreeSet<_>>();
    derived
        .iter()
        .filter(|snapshot| !existing_keys.contains(&(snapshot.theme.to_lowercase(), snapshot.date)))
        .cloned()
        .collect()
}

fn theme_mappings_for(sector_rotation: &[SectorRotationSnapshot]) -> Vec<ThemeMapping> {
    let mut mappings = default_theme_mappings();
    let existing_themes = mappings
        .iter()
        .map(|mapping| mapping.theme.to_lowercase())
        .collect::<BTreeSet<_>>();
    let mapped_sectors = mappings
        .iter()
        .flat_map(|mapping| mapping.sectors.iter())
        .map(|sector| sector.to_lowercase())
        .collect::<BTreeSet<_>>();

    let sector_mappings = sector_rotation
        .iter()
        .filter_map(|snapshot| {
            let sector = snapshot.sector.trim();
            let key = sector.to_lowercase();
            (!sector.is_empty()
                && !existing_themes.contains(&key)
                && !mapped_sectors.contains(&key))
            .then(|| {
                (
                    key,
                    ThemeMapping {
                        theme: sector.to_string(),
                        sectors: vec![sector.to_string()],
                    },
                )
            })
        })
        .collect::<BTreeMap<_, _>>()
        .into_values();

    mappings.extend(sector_mappings);
    mappings
}

fn trend_windows() -> &'static [TrendWindowDef] {
    &[
        TrendWindowDef {
            key: "3d",
            days: Some(3),
        },
        TrendWindowDef {
            key: "5d",
            days: Some(5),
        },
        TrendWindowDef {
            key: "10d",
            days: Some(10),
        },
        TrendWindowDef {
            key: "20d",
            days: Some(20),
        },
        TrendWindowDef {
            key: "history",
            days: None,
        },
    ]
}

fn build_capital_flow_trend(snapshots: &[CapitalFlowSnapshot], days: Option<i64>) -> FlowTrend {
    let filtered = filter_by_last_dates(snapshots, days, |snapshot| snapshot.date);
    let mut groups: TrendGroups = BTreeMap::new();
    for snapshot in filtered {
        groups
            .entry((snapshot.market.clone(), snapshot.category.clone()))
            .or_default()
            .push((
                snapshot.date,
                snapshot.net_flow,
                Some(snapshot.source.clone()),
                None,
            ));
    }

    build_flow_trend_from_groups(groups)
}

fn build_sector_rotation_trend(
    snapshots: &[SectorRotationSnapshot],
    days: Option<i64>,
) -> FlowTrend {
    let filtered = filter_by_last_dates(snapshots, days, |snapshot| snapshot.date);
    let mut groups: TrendGroups = BTreeMap::new();
    for snapshot in filtered {
        let Some(value) = sector_rotation_trend_value(snapshot) else {
            continue;
        };
        groups
            .entry((snapshot.market.clone(), snapshot.sector.clone()))
            .or_default()
            .push((
                snapshot.date,
                value,
                Some(snapshot.source.clone()),
                snapshot.ranking,
            ));
    }

    build_flow_trend_from_groups(groups)
}

fn sector_rotation_trend_value(snapshot: &SectorRotationSnapshot) -> Option<f64> {
    snapshot.net_flow.or(snapshot.change_pct)
}

fn build_theme_rotation_trend(
    snapshots: &[ThemeRotationSnapshot],
    days: Option<i64>,
) -> ThemeTrend {
    let filtered = filter_by_last_dates(snapshots, days, |snapshot| snapshot.date);
    let mut flow_groups: TrendGroups = BTreeMap::new();
    let mut momentum_groups: TrendGroups = BTreeMap::new();

    for snapshot in filtered {
        if let Some(value) = snapshot.flow_score {
            flow_groups
                .entry(("GLOBAL".to_string(), snapshot.theme.clone()))
                .or_default()
                .push((
                    snapshot.date,
                    value,
                    Some("sector_rotation_rules".to_string()),
                    snapshot.ranking,
                ));
        }
        if let Some(value) = snapshot.momentum {
            momentum_groups
                .entry(("GLOBAL".to_string(), snapshot.theme.clone()))
                .or_default()
                .push((
                    snapshot.date,
                    value,
                    Some("sector_rotation_rules".to_string()),
                    snapshot.ranking,
                ));
        }
    }

    let flow = build_flow_trend_from_groups(flow_groups);
    let momentum = build_flow_trend_from_groups(momentum_groups);

    ThemeTrend {
        flow_score_series: flow.series,
        momentum_series: momentum.series,
        top_themes: flow.top_inflows,
        bottom_themes: flow.top_outflows,
    }
}

fn build_flow_trend_from_groups(groups: TrendGroups) -> FlowTrend {
    let mut series = groups
        .into_iter()
        .filter_map(|((market, name), mut rows)| {
            rows.sort_by_key(|row| row.0);
            let mut cumulative = 0.0;
            let points = rows
                .iter()
                .map(|(date, value, _, _)| {
                    cumulative += value;
                    TimeSeriesPoint {
                        date: *date,
                        value: *value,
                        cumulative,
                    }
                })
                .collect::<Vec<_>>();
            let latest_value = rows.last().map(|row| row.1);
            let source = rows.iter().rev().find_map(|row| row.2.clone());
            let latest_ranking = rows.iter().rev().find_map(|row| row.3);
            let latest_date = rows.last().map(|row| row.0);

            (!points.is_empty()).then_some((
                NamedTimeSeries {
                    name,
                    market: (market != "GLOBAL").then_some(market),
                    points,
                    latest_value,
                    cumulative_value: cumulative,
                    source: source.clone(),
                },
                latest_ranking,
                latest_date,
                source,
            ))
        })
        .collect::<Vec<_>>();

    series.sort_by(|a, b| {
        b.0.cumulative_value
            .abs()
            .partial_cmp(&a.0.cumulative_value.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut top_inflows = ranking_items(&series, true);
    let mut top_outflows = ranking_items(&series, false);
    top_inflows.truncate(RANKING_LIMIT);
    top_outflows.truncate(RANKING_LIMIT);

    FlowTrend {
        series: series
            .into_iter()
            .take(SERIES_LIMIT)
            .map(|(series, _, _, _)| series)
            .collect(),
        top_inflows,
        top_outflows,
    }
}

fn ranking_items(series: &[RankedSeries], descending: bool) -> Vec<RankingItem> {
    let mut items = series
        .iter()
        .map(|(series, ranking, date, source)| RankingItem {
            name: series.name.clone(),
            market: series.market.clone(),
            value: series.cumulative_value,
            secondary_value: series.latest_value,
            ranking: *ranking,
            date: *date,
            source: source.clone(),
        })
        .collect::<Vec<_>>();

    items.sort_by(|a, b| {
        let ordering = a
            .value
            .partial_cmp(&b.value)
            .unwrap_or(std::cmp::Ordering::Equal);
        if descending {
            ordering.reverse()
        } else {
            ordering
        }
    });
    items
}

fn filter_by_last_dates<T>(
    snapshots: &[T],
    days: Option<i64>,
    date_fn: impl Fn(&T) -> NaiveDate,
) -> Vec<&T> {
    let Some(days) = days else {
        return snapshots.iter().collect();
    };
    let dates = snapshots
        .iter()
        .map(&date_fn)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .rev()
        .take(days.max(1) as usize)
        .collect::<BTreeSet<_>>();
    snapshots
        .iter()
        .filter(|snapshot| dates.contains(&date_fn(snapshot)))
        .collect()
}

/// Deterministically assess the current market money regime from already
/// collected objective data (sector breadth, aggregate capital flow, connect
/// flows). This is rule-based, not AI/LLM inference.
fn assess_market_regime(summary: &MarketIntelligenceSummary) -> MarketRegimeAssessment {
    let sector_latest_date = summary.sector_rotation.iter().map(|s| s.date).max();
    let capital_latest_date = summary.capital_flow.iter().map(|s| s.date).max();
    let macro_latest_date = summary.macro_capital.iter().map(|s| s.date).max();
    let as_of = [sector_latest_date, capital_latest_date, macro_latest_date]
        .into_iter()
        .flatten()
        .max();

    // Sector breadth and net/gross ratio on the freshest sector date.
    let mut inflow_sectors = 0;
    let mut outflow_sectors = 0;
    let mut net = 0.0;
    let mut gross = 0.0;
    if let Some(date) = sector_latest_date {
        for snapshot in summary.sector_rotation.iter().filter(|s| s.date == date) {
            let Some(value) = snapshot.net_flow.or(snapshot.change_pct) else {
                continue;
            };
            if value > 0.0 {
                inflow_sectors += 1;
            } else if value < 0.0 {
                outflow_sectors += 1;
            }
            net += value;
            gross += value.abs();
        }
    }
    let sector_total = inflow_sectors + outflow_sectors;
    let breadth = (sector_total > 0).then(|| inflow_sectors as f64 / sector_total as f64);
    let net_ratio = (gross > 0.0).then_some(net / gross);

    let main_net_flow = summary
        .capital_flow
        .iter()
        .filter(|s| Some(s.date) == capital_latest_date)
        .map(|s| s.net_flow)
        .reduce(|a, b| a + b);

    let latest_macro = |ind: &str| {
        summary
            .macro_capital
            .iter()
            .filter(|s| s.indicator == ind)
            .max_by_key(|s| s.date)
            .map(|s| s.value)
    };
    let northbound_net_flow = latest_macro(indicator::NORTHBOUND);
    let margin_balance = latest_macro(indicator::MARGIN_BALANCE);

    let metrics = MarketRegimeMetrics {
        breadth,
        net_ratio,
        inflow_sectors,
        outflow_sectors,
        main_net_flow,
        northbound_net_flow,
        margin_balance,
    };

    // No usable input at all -> Unknown (never fabricate a regime).
    if net_ratio.is_none() && main_net_flow.is_none() && northbound_net_flow.is_none() {
        return MarketRegimeAssessment {
            state: regime::UNKNOWN.to_string(),
            rationale: vec!["缺少资金流、板块和北向数据，无法判断当前资金状态。".to_string()],
            as_of,
            sources: Vec::new(),
            data_complete: false,
            metrics,
        };
    }

    let mut rationale = Vec::new();
    let state = if let (Some(net_ratio), Some(breadth)) = (net_ratio, breadth) {
        rationale.push(format!(
            "板块净流入占比 {:.0}%（{} 流入 / {} 流出），净额/总额比 {:+.0}%。",
            breadth * 100.0,
            inflow_sectors,
            outflow_sectors,
            net_ratio * 100.0
        ));
        if net_ratio >= 0.15 && breadth >= 0.55 {
            rationale.push("多数板块净流入且资金整体为正，判定为增量资金进入。".to_string());
            regime::NEW_MONEY
        } else if net_ratio <= -0.15 && breadth <= 0.45 {
            rationale.push("多数板块净流出且资金整体为负，判定为整体流出。".to_string());
            regime::OUTFLOW
        } else if (0.30..=0.70).contains(&breadth) && net_ratio.abs() < 0.15 {
            rationale.push("资金净额接近平衡但板块间强弱分化，判定为板块轮动。".to_string());
            regime::ROTATION
        } else {
            rationale.push("资金活跃度与方向均不显著，判定为中性。".to_string());
            regime::NEUTRAL
        }
    } else if let Some(main_net) = main_net_flow {
        if main_net > 0.0 {
            rationale.push("仅有主力资金净额数据且为净流入，暂判定为增量资金进入。".to_string());
            regime::NEW_MONEY
        } else if main_net < 0.0 {
            rationale.push("仅有主力资金净额数据且为净流出，暂判定为整体流出。".to_string());
            regime::OUTFLOW
        } else {
            regime::NEUTRAL
        }
    } else if let Some(north) = northbound_net_flow {
        if north > 0.0 {
            regime::NEW_MONEY
        } else if north < 0.0 {
            regime::OUTFLOW
        } else {
            regime::NEUTRAL
        }
    } else {
        regime::NEUTRAL
    };

    if let Some(north) = northbound_net_flow {
        rationale.push(format!("北向资金净流入 {north:+.2} 亿元。"));
    }
    if let Some(margin) = margin_balance {
        rationale.push(format!("两融余额 {margin:.0} 亿元。"));
    }

    let mut sources = Vec::new();
    for snapshot in &summary.sector_rotation {
        push_unique(&mut sources, &snapshot.source);
    }
    for snapshot in &summary.capital_flow {
        push_unique(&mut sources, &snapshot.source);
    }
    for snapshot in &summary.macro_capital {
        push_unique(&mut sources, &snapshot.source);
    }

    let data_complete =
        net_ratio.is_some() && main_net_flow.is_some() && northbound_net_flow.is_some();

    MarketRegimeAssessment {
        state: state.to_string(),
        rationale,
        as_of,
        sources,
        data_complete,
        metrics,
    }
}

fn push_unique(items: &mut Vec<String>, value: &str) {
    if !value.is_empty() && !items.iter().any(|item| item == value) {
        items.push(value.to_string());
    }
}

struct MacroIndex {
    indicator: &'static str,
    market: &'static str,
    symbol: &'static str,
    fallback_symbols: &'static [&'static str],
    currency: &'static str,
}

fn macro_indices() -> &'static [MacroIndex] {
    &[
        MacroIndex {
            indicator: indicator::DXY,
            market: "GLOBAL",
            symbol: "DX-Y.NYB",
            fallback_symbols: &["DX=F"],
            currency: "USD",
        },
        MacroIndex {
            indicator: indicator::VIX,
            market: "US",
            symbol: "^VIX",
            fallback_symbols: &[],
            currency: "USD",
        },
    ]
}

fn data_status(summary: &MarketIntelligenceSummary) -> Vec<MarketIntelligenceDataStatus> {
    vec![
        status(
            "marketOverview",
            !summary.market_overview.is_empty(),
            "yahoo_alpha_vantage",
        ),
        status("capitalFlow", !summary.capital_flow.is_empty(), "eastmoney"),
        status(
            "sectorRotation",
            !summary.sector_rotation.is_empty(),
            "eastmoney",
        ),
        status(
            "themeRotation",
            !summary.theme_rotation.is_empty(),
            "sector_rotation_rules",
        ),
        status(
            "portfolioExposure",
            !summary.portfolio_exposure.is_empty(),
            "fund_research_lookthrough",
        ),
        status(
            "macroCapital",
            !summary.macro_capital.is_empty(),
            "eastmoney_datacenter/yahoo",
        ),
        status(
            "flowSignal",
            summary.flow_signal.state != flow_state::UNKNOWN,
            "market_flow_signal_service",
        ),
    ]
}

fn status(section: &str, available: bool, source: &str) -> MarketIntelligenceDataStatus {
    MarketIntelligenceDataStatus {
        section: section.to_string(),
        available,
        message: if available {
            None
        } else {
            Some("真实数据源暂不可用或尚未产生快照。".to_string())
        },
        source: Some(source.to_string()),
    }
}

struct BenchmarkIndex {
    market: &'static str,
    name: &'static str,
    symbol: &'static str,
    fallback_symbols: &'static [&'static str],
    currency: &'static str,
}

async fn fetch_first_available_benchmark_quotes(
    quote_service: &dyn QuoteServiceTrait,
    benchmark: &BenchmarkIndex,
    start: chrono::NaiveDate,
    end: chrono::NaiveDate,
) -> Result<Option<(&'static str, Vec<crate::quotes::Quote>)>> {
    for symbol in
        std::iter::once(benchmark.symbol).chain(benchmark.fallback_symbols.iter().copied())
    {
        match quote_service
            .fetch_quotes_for_symbol_with_provider(
                symbol,
                benchmark.currency,
                Some(DATA_SOURCE_YAHOO),
                start,
                end,
            )
            .await
        {
            Ok(quotes) if !quotes.is_empty() => return Ok(Some((symbol, quotes))),
            Ok(_) => {
                warn!("Benchmark quote {symbol} returned no data for market intelligence");
            }
            Err(error) => {
                warn!("Failed to fetch benchmark quote {symbol} for market intelligence: {error}");
            }
        }
    }

    Ok(None)
}

fn benchmark_indices() -> &'static [BenchmarkIndex] {
    &[
        BenchmarkIndex {
            market: "CN",
            name: "上证指数",
            symbol: "000001.SS",
            fallback_symbols: &[],
            currency: "CNY",
        },
        BenchmarkIndex {
            market: "CN",
            name: "深证成指",
            symbol: "399001.SZ",
            fallback_symbols: &[],
            currency: "CNY",
        },
        BenchmarkIndex {
            market: "CN",
            name: "创业板指",
            symbol: "399006.SZ",
            fallback_symbols: &[],
            currency: "CNY",
        },
        BenchmarkIndex {
            market: "CN",
            name: "科创50",
            symbol: "000688.SS",
            fallback_symbols: &[],
            currency: "CNY",
        },
        BenchmarkIndex {
            market: "HK",
            name: "恒生指数",
            symbol: "^HSI",
            fallback_symbols: &["2800.HK"],
            currency: "HKD",
        },
        BenchmarkIndex {
            market: "HK",
            name: "恒生科技",
            symbol: "^HSTECH",
            fallback_symbols: &["3067.HK", "3032.HK"],
            currency: "HKD",
        },
        BenchmarkIndex {
            market: "US",
            name: "S&P 500",
            symbol: "^GSPC",
            fallback_symbols: &["SPY"],
            currency: "USD",
        },
        BenchmarkIndex {
            market: "US",
            name: "NASDAQ",
            symbol: "^IXIC",
            fallback_symbols: &["QQQ"],
            currency: "USD",
        },
        BenchmarkIndex {
            market: "US",
            name: "DOW",
            symbol: "^DJI",
            fallback_symbols: &["DIA"],
            currency: "USD",
        },
        BenchmarkIndex {
            market: "US",
            name: "Russell 2000",
            symbol: "^RUT",
            fallback_symbols: &["IWM"],
            currency: "USD",
        },
    ]
}
