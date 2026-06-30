use std::sync::Arc;

use chrono::{Duration, Utc};
use log::warn;
use rust_decimal::prelude::ToPrimitive;

use super::{
    capital_flow::{
        CapitalFlowProvider, CapitalFlowRepository, CapitalFlowService, CapitalFlowSnapshot,
    },
    eastmoney::{default_theme_mappings, EastmoneyMarketIntelligenceProvider},
    market_overview::{
        MarketDataProvider, MarketOverviewService, MarketSnapshot, MarketSnapshotRepository,
    },
    portfolio_exposure::{
        PortfolioExposureService, PortfolioThemeExposure, PortfolioThemeExposureRepository,
    },
    sector_rotation::{
        SectorRotationProvider, SectorRotationRepository, SectorRotationService,
        SectorRotationSnapshot,
    },
    theme_rotation::{ThemeRotationRepository, ThemeRotationService, ThemeRotationSnapshot},
};
use crate::{errors::Result, fund_research::FundResearchService, quotes::QuoteServiceTrait};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarketIntelligenceSummary {
    pub market_overview: Vec<MarketSnapshot>,
    pub capital_flow: Vec<CapitalFlowSnapshot>,
    pub sector_rotation: Vec<SectorRotationSnapshot>,
    pub theme_rotation: Vec<ThemeRotationSnapshot>,
    pub portfolio_exposure: Vec<PortfolioThemeExposure>,
    pub data_status: Vec<MarketIntelligenceDataStatus>,
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
        portfolio_exposure_repository: Arc<dyn PortfolioThemeExposureRepository>,
    ) -> Self {
        Self {
            market_overview: MarketOverviewService::new(market_overview_repository),
            capital_flow: CapitalFlowService::new(capital_flow_repository),
            sector_rotation: SectorRotationService::new(sector_rotation_repository),
            theme_rotation: ThemeRotationService::new(theme_rotation_repository),
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
        let portfolio_exposure = match portfolio_id {
            Some(portfolio_id) => {
                self.latest_or_calculate_portfolio_exposure(portfolio_id)
                    .await?
            }
            None => Vec::new(),
        };
        let summary = MarketIntelligenceSummary {
            market_overview: self.market_overview.latest(None).await?,
            capital_flow: self.capital_flow.snapshots(None, None, None, 100).await?,
            sector_rotation: self
                .sector_rotation
                .snapshots(None, None, None, 100)
                .await?,
            theme_rotation: self.theme_rotation.snapshots(None, None, 100).await?,
            portfolio_exposure,
            data_status: Vec::new(),
        };

        if summary.has_market_data() {
            return Ok(with_data_status(summary));
        }

        match self.refresh_eastmoney().await {
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

    pub async fn refresh_eastmoney(&self) -> Result<MarketIntelligenceSummary> {
        let provider = EastmoneyMarketIntelligenceProvider::default();
        let mut market_overview = self.fetch_benchmark_market_snapshots().await?;
        match provider.fetch_market_snapshots().await {
            Ok(snapshots) => merge_market_overview(&mut market_overview, snapshots),
            Err(error) => warn!("Failed to fetch Eastmoney market overview: {error}"),
        }

        if !market_overview.is_empty() {
            self.market_overview
                .ingest_snapshots(market_overview.clone())
                .await?;
        }

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
                .calculate_and_save(default_theme_mappings(), &sector_rotation)
                .await?
        };

        Ok(with_data_status(MarketIntelligenceSummary {
            market_overview,
            capital_flow,
            sector_rotation,
            theme_rotation,
            portfolio_exposure: Vec::new(),
            data_status: Vec::new(),
        }))
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
                .calculate_and_save(default_theme_mappings(), &sector_rotation)
                .await?
        };

        Ok(with_data_status(MarketIntelligenceSummary {
            market_overview,
            capital_flow,
            sector_rotation,
            theme_rotation,
            portfolio_exposure: Vec::new(),
            data_status: Vec::new(),
        }))
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
            let quotes = match quote_service
                .fetch_quotes_for_symbol(benchmark.symbol, benchmark.currency, start, end)
                .await
            {
                Ok(quotes) => quotes,
                Err(error) => {
                    warn!(
                        "Failed to fetch benchmark quote {} for market intelligence: {}",
                        benchmark.symbol, error
                    );
                    continue;
                }
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
                source: "benchmark_quotes".to_string(),
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

fn with_data_status(mut summary: MarketIntelligenceSummary) -> MarketIntelligenceSummary {
    summary.data_status = data_status(&summary);
    summary
}

fn data_status(summary: &MarketIntelligenceSummary) -> Vec<MarketIntelligenceDataStatus> {
    vec![
        status(
            "marketOverview",
            !summary.market_overview.is_empty(),
            "benchmark_quotes",
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

fn merge_market_overview(current: &mut Vec<MarketSnapshot>, fallback: Vec<MarketSnapshot>) {
    for snapshot in fallback {
        let exists = current
            .iter()
            .any(|item| item.market == snapshot.market && item.index_name == snapshot.index_name);
        if !exists {
            current.push(snapshot);
        }
    }
}

struct BenchmarkIndex {
    market: &'static str,
    name: &'static str,
    symbol: &'static str,
    currency: &'static str,
}

fn benchmark_indices() -> &'static [BenchmarkIndex] {
    &[
        BenchmarkIndex {
            market: "CN",
            name: "上证指数",
            symbol: "000001.SS",
            currency: "CNY",
        },
        BenchmarkIndex {
            market: "CN",
            name: "深证成指",
            symbol: "399001.SZ",
            currency: "CNY",
        },
        BenchmarkIndex {
            market: "CN",
            name: "创业板指",
            symbol: "399006.SZ",
            currency: "CNY",
        },
        BenchmarkIndex {
            market: "CN",
            name: "科创50",
            symbol: "000688.SS",
            currency: "CNY",
        },
        BenchmarkIndex {
            market: "HK",
            name: "恒生指数",
            symbol: "^HSI",
            currency: "HKD",
        },
        BenchmarkIndex {
            market: "HK",
            name: "恒生科技",
            symbol: "^HSTECH",
            currency: "HKD",
        },
        BenchmarkIndex {
            market: "US",
            name: "S&P 500",
            symbol: "^GSPC",
            currency: "USD",
        },
        BenchmarkIndex {
            market: "US",
            name: "NASDAQ",
            symbol: "^IXIC",
            currency: "USD",
        },
        BenchmarkIndex {
            market: "US",
            name: "DOW",
            symbol: "^DJI",
            currency: "USD",
        },
        BenchmarkIndex {
            market: "US",
            name: "Russell 2000",
            symbol: "^RUT",
            currency: "USD",
        },
    ]
}
