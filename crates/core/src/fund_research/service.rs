use std::collections::BTreeMap;
use std::sync::Arc;

use crate::errors::Result;

use super::alerts::generate_rebalance_alerts;
use super::lookthrough::{
    fund_top_holdings, lookthrough_theme_exposure, portfolio_fund_lookthrough,
};
use super::metrics::holding_concentration;
use super::models::{
    AnnouncementCategory, AnnouncementImportance, AssetAllocation, FundAiFeatures,
    FundAnnouncement, FundHoldingChange, FundInternalHolding, FundNavPoint, FundPerformance,
    FundQuote, FundResearchSnapshot, FundRiskMetrics, FundTopHolding, HoldingAssetType,
    HoldingChangeType, PortfolioFundLookthroughSummary, PortfolioThemeExposure, RebalanceAlert,
    RegionAllocation, SectorAllocation, SectorRotationSignal, StockClassificationOverride,
    UpsertStockClassificationOverride,
};
use super::repository::{
    FundResearchFetcher, FundResearchRepository, PortfolioFundPositionProvider,
};
use super::rotation::analyze_theme_rotation_from_snapshots;
use super::theme_mapping::{calculate_theme_exposures, ThemeMappingConfig};

pub struct EastmoneyFundResearchFetcher {
    provider: wealthfolio_market_data::EastmoneyFundProvider,
}

impl EastmoneyFundResearchFetcher {
    pub fn new(provider: wealthfolio_market_data::EastmoneyFundProvider) -> Self {
        Self { provider }
    }
}

#[async_trait::async_trait]
impl FundResearchFetcher for EastmoneyFundResearchFetcher {
    async fn fetch_fund_research(&self, fund_code: &str) -> Result<FundResearchSnapshot> {
        let snapshot = self
            .provider
            .fetch_fund_snapshot(fund_code)
            .await
            .map_err(|error| crate::errors::Error::Repository(error.to_string()))?;
        Ok(map_eastmoney_snapshot(snapshot))
    }
}

pub struct FundResearchService {
    repository: Arc<dyn FundResearchRepository>,
    fetcher: Arc<dyn FundResearchFetcher>,
    position_provider: Option<Arc<dyn PortfolioFundPositionProvider>>,
    theme_mapping: ThemeMappingConfig,
}

impl FundResearchService {
    pub fn new(
        repository: Arc<dyn FundResearchRepository>,
        fetcher: Arc<dyn FundResearchFetcher>,
        theme_mapping: ThemeMappingConfig,
    ) -> Self {
        Self {
            repository,
            fetcher,
            position_provider: None,
            theme_mapping,
        }
    }

    pub fn with_position_provider(
        mut self,
        position_provider: Arc<dyn PortfolioFundPositionProvider>,
    ) -> Self {
        self.position_provider = Some(position_provider);
        self
    }

    pub async fn refresh_fund_research(&self, fund_code: &str) -> Result<FundResearchSnapshot> {
        let mut snapshot = self.fetcher.fetch_fund_research(fund_code).await?;
        self.apply_stock_classification_overrides(&mut snapshot)
            .await?;
        enrich_snapshot(&mut snapshot, &self.theme_mapping);

        let mut previous = self.repository.latest_snapshot(&snapshot.fund_code).await?;
        if let Some(previous_snapshot) = previous.as_mut() {
            self.apply_stock_classification_overrides(previous_snapshot)
                .await?;
            enrich_snapshot(previous_snapshot, &self.theme_mapping);
        }
        let alerts = generate_rebalance_alerts(&snapshot, previous.as_ref());

        self.repository.upsert_snapshot(&snapshot).await?;
        self.repository.upsert_rebalance_alerts(&alerts).await?;
        Ok(snapshot)
    }

    pub async fn get_fund_snapshot(&self, fund_code: &str) -> Result<Option<FundResearchSnapshot>> {
        let Some(mut snapshot) = self.repository.latest_snapshot(fund_code).await? else {
            return Ok(None);
        };
        self.apply_stock_classification_overrides(&mut snapshot)
            .await?;
        enrich_snapshot(&mut snapshot, &self.theme_mapping);
        Ok(Some(snapshot))
    }

    pub async fn analyze_theme_rotation(
        &self,
        themes: Vec<String>,
    ) -> Result<Vec<SectorRotationSignal>> {
        let mut snapshots = Vec::new();
        for theme in &themes {
            snapshots.extend(self.repository.snapshots_for_theme(theme, 100).await?);
        }
        let signals = analyze_theme_rotation_from_snapshots(&themes, &snapshots);
        self.repository.upsert_rotation_signals(&signals).await?;
        Ok(signals)
    }

    pub async fn get_rebalance_alerts(
        &self,
        fund_code: Option<&str>,
    ) -> Result<Vec<RebalanceAlert>> {
        self.repository.rebalance_alerts(fund_code).await
    }

    pub async fn analyze_portfolio_theme_exposure(
        &self,
        portfolio_id: &str,
    ) -> Result<Vec<PortfolioThemeExposure>> {
        let Some(position_provider) = &self.position_provider else {
            return Err(crate::errors::Error::Repository(
                "Portfolio fund position provider is not configured".to_string(),
            ));
        };
        let positions = position_provider.fund_positions(portfolio_id).await?;
        let fund_codes: Vec<String> = positions
            .iter()
            .map(|position| position.fund_code.clone())
            .collect();
        let mut snapshots = self
            .repository
            .latest_snapshots_for_funds(&fund_codes)
            .await?;
        for snapshot in &mut snapshots {
            self.apply_stock_classification_overrides(snapshot).await?;
            enrich_snapshot(snapshot, &self.theme_mapping);
        }
        Ok(lookthrough_theme_exposure(
            portfolio_id,
            &positions,
            &snapshots,
        ))
    }

    /// Get the top 10 holdings for a single fund.
    pub async fn get_fund_top_holdings(&self, fund_code: &str) -> Result<Vec<FundTopHolding>> {
        let Some(snapshot) = self.repository.latest_snapshot(fund_code).await? else {
            return Ok(Vec::new());
        };
        let mut snapshot = snapshot;
        self.apply_stock_classification_overrides(&mut snapshot)
            .await?;
        enrich_snapshot(&mut snapshot, &self.theme_mapping);
        Ok(fund_top_holdings(&snapshot))
    }

    /// Analyze portfolio fund lookthrough: aggregate underlying holdings
    /// across all user fund positions.
    pub async fn analyze_portfolio_fund_lookthrough(
        &self,
        portfolio_id: &str,
    ) -> Result<PortfolioFundLookthroughSummary> {
        let Some(position_provider) = &self.position_provider else {
            return Err(crate::errors::Error::Repository(
                "Portfolio fund position provider is not configured".to_string(),
            ));
        };
        let positions = position_provider.fund_positions(portfolio_id).await?;
        let fund_codes: Vec<String> = positions
            .iter()
            .map(|position| position.fund_code.clone())
            .collect();
        let mut snapshots = self
            .repository
            .latest_snapshots_for_funds(&fund_codes)
            .await?;
        for snapshot in &mut snapshots {
            self.apply_stock_classification_overrides(snapshot).await?;
            enrich_snapshot(snapshot, &self.theme_mapping);
        }
        Ok(portfolio_fund_lookthrough(
            portfolio_id,
            &positions,
            &snapshots,
        ))
    }

    pub async fn get_stock_classification_overrides(
        &self,
    ) -> Result<Vec<StockClassificationOverride>> {
        self.repository.stock_classification_overrides().await
    }

    pub async fn save_stock_classification_override(
        &self,
        input: UpsertStockClassificationOverride,
    ) -> Result<StockClassificationOverride> {
        self.repository
            .upsert_stock_classification_override(input)
            .await
    }

    pub async fn delete_stock_classification_override(&self, stock_key: &str) -> Result<()> {
        self.repository
            .delete_stock_classification_override(stock_key)
            .await
    }

    async fn apply_stock_classification_overrides(
        &self,
        snapshot: &mut FundResearchSnapshot,
    ) -> Result<()> {
        let overrides = self.repository.stock_classification_overrides().await?;
        let by_key: BTreeMap<String, StockClassificationOverride> = overrides
            .into_iter()
            .map(|override_item| (override_item.stock_key.clone(), override_item))
            .collect();

        for holding in &mut snapshot.fund_holdings {
            let key = stock_classification_key(holding.asset_code.as_deref(), &holding.asset_name);
            if let Some(override_item) = by_key.get(&key) {
                apply_stock_classification_override(holding, override_item);
            }
        }
        Ok(())
    }
}

fn apply_stock_classification_override(
    holding: &mut FundInternalHolding,
    override_item: &StockClassificationOverride,
) {
    holding.sector = override_item.sector.clone();
    holding.industry = override_item.industry.clone();
    holding.theme_tags = override_item.theme_tags.clone();
}

pub fn stock_classification_key(asset_code: Option<&str>, asset_name: &str) -> String {
    let key = match asset_code.filter(|code| !code.trim().is_empty()) {
        Some(code) => code.trim().to_string(),
        None => clean_html(asset_name).trim().to_string(),
    };
    key.to_uppercase()
}

fn enrich_snapshot(snapshot: &mut FundResearchSnapshot, mapping: &ThemeMappingConfig) {
    snapshot.theme_exposures = calculate_theme_exposures(
        &snapshot.fund_code,
        snapshot.snapshot_date,
        &snapshot.fund_holdings,
        &snapshot.sector_allocations,
        mapping,
    );
    snapshot.ai_features.holding_concentration =
        Some(holding_concentration(&snapshot.fund_holdings));
}

/// Strip HTML tags and decode common entities from Eastmoney scraped text.
fn clean_html(input: &str) -> String {
    // Remove HTML tags like <a class='tol'>...</a>
    let s = regex::Regex::new(r"<[^>]*>")
        .unwrap()
        .replace_all(input, "");
    // Trim whitespace
    s.trim().to_string()
}

/// Try to infer a stock code from the cleaned asset name when
/// Eastmoney doesn't provide one (common for US stocks in fund holdings).
/// e.g. "GOOG" → Some("GOOG"), "中际旭创" → None
fn infer_stock_code_from_name(name: &str) -> Option<String> {
    let trimmed = name.trim();
    // Match 1-5 letter alphabetic ticker (e.g. AAPL, GOOG, BRK.B)
    if let Some(captured) = regex::Regex::new(r"^([A-Z]{1,5}(\.[A-Z]{1,3})?)$")
        .unwrap()
        .captures(trimmed)
    {
        return Some(captured[1].to_string());
    }
    None
}

fn resolve_asset_code(stock_code: Option<String>, stock_name: &str) -> Option<String> {
    stock_code.or_else(|| infer_stock_code_from_name(stock_name))
}

fn map_eastmoney_snapshot(
    snapshot: wealthfolio_market_data::FundResearchSnapshot,
) -> FundResearchSnapshot {
    let snapshot_date = snapshot
        .quote
        .as_ref()
        .map(|quote| quote.nav_date)
        .or_else(|| snapshot.nav_history.last().map(|point| point.date))
        .unwrap_or_else(|| snapshot.fetched_at.date_naive());
    let fund_code = snapshot.code;

    FundResearchSnapshot {
        fund_code: fund_code.clone(),
        fund_name: snapshot.name,
        snapshot_date,
        source: snapshot.source,
        quote: snapshot.quote.map(|quote| FundQuote {
            nav: quote.nav,
            nav_date: quote.nav_date,
            daily_return_pct: quote.daily_return_pct,
        }),
        nav_history: snapshot
            .nav_history
            .into_iter()
            .map(|point| FundNavPoint {
                date: point.date,
                nav: point.nav,
                daily_return_pct: point.daily_return_pct,
            })
            .collect(),
        fund_holdings: snapshot
            .holdings
            .into_iter()
            .map(|holding| FundInternalHolding {
                fund_code: fund_code.clone(),
                report_date: holding.report_date.unwrap_or(snapshot_date),
                rank: holding.rank,
                asset_code: resolve_asset_code(
                    holding.stock_code,
                    &clean_html(&holding.stock_name),
                ),
                asset_name: clean_html(&holding.stock_name),
                asset_type: HoldingAssetType::Stock,
                market: holding.market,
                sector: holding.industry.clone(),
                industry: holding.industry,
                weight_pct: holding.weight_pct,
                theme_tags: holding.concept_tags,
            })
            .collect(),
        holding_changes: snapshot
            .holding_changes
            .into_iter()
            .map(|change| FundHoldingChange {
                fund_code: fund_code.clone(),
                report_date: snapshot_date,
                asset_code: resolve_asset_code(change.stock_code, &clean_html(&change.stock_name)),
                asset_name: clean_html(&change.stock_name),
                previous_weight_pct: change.previous_weight_pct,
                current_weight_pct: change.current_weight_pct,
                weight_change_pct: change.weight_change_pct,
                previous_rank: change.previous_rank,
                current_rank: change.current_rank,
                change_type: match change.change_type {
                    wealthfolio_market_data::HoldingChangeType::New => HoldingChangeType::New,
                    wealthfolio_market_data::HoldingChangeType::Removed => {
                        HoldingChangeType::Removed
                    }
                    wealthfolio_market_data::HoldingChangeType::Increased => {
                        HoldingChangeType::Increased
                    }
                    wealthfolio_market_data::HoldingChangeType::Decreased => {
                        HoldingChangeType::Decreased
                    }
                    wealthfolio_market_data::HoldingChangeType::Unchanged => {
                        HoldingChangeType::Unchanged
                    }
                },
            })
            .collect(),
        sector_allocations: snapshot
            .sector_allocations
            .into_iter()
            .map(|sector| SectorAllocation {
                fund_code: fund_code.clone(),
                snapshot_date: sector.report_date.unwrap_or(snapshot_date),
                sector: sector.sector,
                weight_pct: sector.weight_pct,
                previous_weight_pct: sector.previous_weight_pct,
                weight_change_pct: sector.weight_change_pct,
            })
            .collect(),
        theme_exposures: Vec::new(),
        asset_allocations: snapshot
            .asset_allocations
            .into_iter()
            .map(|asset| AssetAllocation {
                fund_code: fund_code.clone(),
                snapshot_date: asset.report_date.unwrap_or(snapshot_date),
                asset_class: asset.asset_class,
                weight_pct: asset.weight_pct,
            })
            .collect(),
        region_allocations: snapshot
            .region_allocations
            .into_iter()
            .map(|region| RegionAllocation {
                fund_code: fund_code.clone(),
                snapshot_date: region.report_date.unwrap_or(snapshot_date),
                region: region.region,
                weight_pct: region.weight_pct,
            })
            .collect(),
        performance: snapshot.performance.map(|performance| FundPerformance {
            return_1m_pct: performance.return_1m_pct,
            return_3m_pct: performance.return_3m_pct,
            return_6m_pct: performance.return_6m_pct,
            return_1y_pct: performance.return_1y_pct,
            peer_rank: performance.peer_rank,
        }),
        risk_metrics: snapshot.risk_metrics.map(|risk| FundRiskMetrics {
            max_drawdown_1m_pct: risk.max_drawdown_1m_pct,
            max_drawdown_1y_pct: risk.max_drawdown_1y_pct,
            annualized_volatility_pct: risk.annualized_volatility_pct,
            sharpe_ratio: risk.sharpe_ratio,
        }),
        announcements: snapshot
            .announcements
            .into_iter()
            .map(|announcement| FundAnnouncement {
                fund_code: fund_code.clone(),
                title: announcement.title,
                date: announcement.date,
                url: announcement.url,
                category: announcement.category.map(|category| match category {
                    wealthfolio_market_data::AnnouncementCategory::ManagerChange => {
                        AnnouncementCategory::ManagerChange
                    }
                    wealthfolio_market_data::AnnouncementCategory::PurchaseSuspension => {
                        AnnouncementCategory::PurchaseSuspension
                    }
                    wealthfolio_market_data::AnnouncementCategory::RiskWarning => {
                        AnnouncementCategory::RiskWarning
                    }
                    wealthfolio_market_data::AnnouncementCategory::MajorEvent => {
                        AnnouncementCategory::MajorEvent
                    }
                    _ => AnnouncementCategory::Other,
                }),
                importance: match announcement.importance {
                    wealthfolio_market_data::AnnouncementImportance::Low => {
                        AnnouncementImportance::Low
                    }
                    wealthfolio_market_data::AnnouncementImportance::Medium => {
                        AnnouncementImportance::Medium
                    }
                    wealthfolio_market_data::AnnouncementImportance::High => {
                        AnnouncementImportance::High
                    }
                },
            })
            .collect(),
        ai_features: FundAiFeatures {
            holding_concentration: None,
        },
        created_at: snapshot.fetched_at,
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use chrono::{NaiveDate, Utc};
    use tokio::sync::Mutex;

    use super::*;
    use crate::fund_research::models::PortfolioFundPosition;
    use crate::fund_research::models::{FundAiFeatures, FundInternalHolding, HoldingAssetType};

    struct MemoryRepository {
        snapshots: Mutex<Vec<FundResearchSnapshot>>,
        alerts: Mutex<Vec<RebalanceAlert>>,
        stock_overrides: Mutex<Vec<StockClassificationOverride>>,
    }

    #[async_trait]
    impl FundResearchRepository for MemoryRepository {
        async fn upsert_snapshot(&self, snapshot: &FundResearchSnapshot) -> Result<()> {
            self.snapshots.lock().await.push(snapshot.clone());
            Ok(())
        }

        async fn latest_snapshot(&self, fund_code: &str) -> Result<Option<FundResearchSnapshot>> {
            Ok(self
                .snapshots
                .lock()
                .await
                .iter()
                .rev()
                .find(|snapshot| snapshot.fund_code == fund_code)
                .cloned())
        }

        async fn snapshots_for_theme(
            &self,
            _theme: &str,
            _limit: usize,
        ) -> Result<Vec<FundResearchSnapshot>> {
            Ok(self.snapshots.lock().await.clone())
        }

        async fn latest_snapshots_for_funds(
            &self,
            fund_codes: &[String],
        ) -> Result<Vec<FundResearchSnapshot>> {
            Ok(self
                .snapshots
                .lock()
                .await
                .iter()
                .filter(|snapshot| fund_codes.contains(&snapshot.fund_code))
                .cloned()
                .collect())
        }

        async fn upsert_rotation_signals(&self, _signals: &[SectorRotationSignal]) -> Result<()> {
            Ok(())
        }

        async fn rotation_signals(&self, _themes: &[String]) -> Result<Vec<SectorRotationSignal>> {
            Ok(vec![])
        }

        async fn upsert_rebalance_alerts(&self, alerts: &[RebalanceAlert]) -> Result<()> {
            self.alerts.lock().await.extend_from_slice(alerts);
            Ok(())
        }

        async fn rebalance_alerts(&self, _fund_code: Option<&str>) -> Result<Vec<RebalanceAlert>> {
            Ok(self.alerts.lock().await.clone())
        }

        async fn stock_classification_overrides(&self) -> Result<Vec<StockClassificationOverride>> {
            Ok(self.stock_overrides.lock().await.clone())
        }

        async fn upsert_stock_classification_override(
            &self,
            input: UpsertStockClassificationOverride,
        ) -> Result<StockClassificationOverride> {
            let stock_key =
                stock_classification_key(input.asset_code.as_deref(), &input.asset_name);
            let override_item = StockClassificationOverride {
                stock_key: stock_key.clone(),
                asset_code: input
                    .asset_code
                    .and_then(|value| non_empty_trimmed_string(&value)),
                asset_name: input.asset_name.trim().to_string(),
                sector: input
                    .sector
                    .and_then(|value| non_empty_trimmed_string(&value)),
                industry: input
                    .industry
                    .and_then(|value| non_empty_trimmed_string(&value)),
                theme_tags: input
                    .theme_tags
                    .into_iter()
                    .filter_map(|value| non_empty_trimmed_string(&value))
                    .collect(),
                source: "manual".to_string(),
                updated_at: Utc::now(),
            };
            let mut overrides = self.stock_overrides.lock().await;
            overrides.retain(|existing| existing.stock_key != stock_key);
            overrides.push(override_item.clone());
            Ok(override_item)
        }

        async fn delete_stock_classification_override(&self, stock_key: &str) -> Result<()> {
            let stock_key = stock_key.trim().to_uppercase();
            self.stock_overrides
                .lock()
                .await
                .retain(|existing| existing.stock_key != stock_key);
            Ok(())
        }
    }

    fn non_empty_trimmed_string(value: &str) -> Option<String> {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    }

    struct FixtureFetcher;

    #[async_trait]
    impl FundResearchFetcher for FixtureFetcher {
        async fn fetch_fund_research(&self, fund_code: &str) -> Result<FundResearchSnapshot> {
            let date = NaiveDate::from_ymd_opt(2026, 6, 27).unwrap();
            Ok(FundResearchSnapshot {
                fund_code: fund_code.to_string(),
                fund_name: "Fixture Fund".to_string(),
                snapshot_date: date,
                source: "fixture".to_string(),
                quote: None,
                nav_history: vec![],
                fund_holdings: vec![FundInternalHolding {
                    fund_code: fund_code.to_string(),
                    report_date: date,
                    rank: Some(1),
                    asset_code: None,
                    asset_name: "中际旭创".to_string(),
                    asset_type: HoldingAssetType::Stock,
                    market: None,
                    sector: None,
                    industry: None,
                    weight_pct: Some(10.0),
                    theme_tags: vec![],
                }],
                holding_changes: vec![],
                sector_allocations: vec![],
                theme_exposures: vec![],
                asset_allocations: vec![],
                region_allocations: vec![],
                performance: None,
                risk_metrics: None,
                announcements: vec![],
                ai_features: FundAiFeatures::default(),
                created_at: Utc::now(),
            })
        }
    }

    struct FixturePositions;

    #[async_trait]
    impl PortfolioFundPositionProvider for FixturePositions {
        async fn fund_positions(&self, portfolio_id: &str) -> Result<Vec<PortfolioFundPosition>> {
            Ok(vec![PortfolioFundPosition {
                portfolio_id: portfolio_id.to_string(),
                fund_code: "014002".to_string(),
                market_value_base: 100_000.0,
            }])
        }
    }

    #[tokio::test]
    async fn refresh_and_portfolio_exposure_work_with_missing_optional_data() {
        let repository = Arc::new(MemoryRepository {
            snapshots: Mutex::new(vec![]),
            alerts: Mutex::new(vec![]),
            stock_overrides: Mutex::new(vec![]),
        });
        let mapping = ThemeMappingConfig::from_toml(
            r#"
            [theme_mapping.OpticalModule]
            holdings = ["中际旭创"]
            "#,
        )
        .unwrap();
        let service = FundResearchService::new(repository, Arc::new(FixtureFetcher), mapping)
            .with_position_provider(Arc::new(FixturePositions));

        service.refresh_fund_research("014002").await.unwrap();
        let exposure = service
            .analyze_portfolio_theme_exposure("p1")
            .await
            .unwrap();
        assert_eq!(exposure[0].theme, "OpticalModule");
        assert_eq!(exposure[0].exposure_value_base, 10_000.0);
    }
}
