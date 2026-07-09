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
    intraday::{
        IntradayMarketIntelligenceProvider, MarketIntelligenceIntradayRepository,
        MarketIntelligenceIntradayService,
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
    pub trends: MarketIntelligenceTrends,
    pub data_status: Vec<MarketIntelligenceDataStatus>,
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
    ) -> Self {
        Self {
            market_overview: MarketOverviewService::new(market_overview_repository),
            capital_flow: CapitalFlowService::new(capital_flow_repository),
            sector_rotation: SectorRotationService::new(sector_rotation_repository),
            theme_rotation: ThemeRotationService::new(theme_rotation_repository),
            intraday: MarketIntelligenceIntradayService::new(intraday_repository),
            portfolio_exposure: PortfolioExposureService::new(portfolio_exposure_repository),
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
            trends: self.trends().await?,
            data_status: Vec::new(),
        };

        if summary.has_market_data() {
            return Ok(with_data_status(summary));
        }

        match self.refresh_market_data().await {
            Ok(mut refreshed) => {
                if let Some(portfolio_id) = portfolio_id {
                    refreshed.portfolio_exposure = self
                        .latest_or_calculate_portfolio_exposure(portfolio_id)
                        .await?;
                }
                refreshed.data_status = data_status(&refreshed);
                Ok(refreshed)
            }
            Err(error) => {
                warn!("Failed to auto-refresh market intelligence summary: {error}");
                Ok(with_data_status(summary))
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

        Ok(with_data_status(MarketIntelligenceSummary {
            market_overview,
            capital_flow,
            sector_rotation,
            theme_rotation,
            portfolio_exposure: Vec::new(),
            trends: self.trends().await?,
            data_status: Vec::new(),
        }))
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

        Ok(with_data_status(MarketIntelligenceSummary {
            market_overview,
            capital_flow,
            sector_rotation,
            theme_rotation,
            portfolio_exposure: Vec::new(),
            trends: self.trends().await?,
            data_status: Vec::new(),
        }))
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

fn with_data_status(mut summary: MarketIntelligenceSummary) -> MarketIntelligenceSummary {
    summary.data_status = data_status(&summary);
    summary
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
