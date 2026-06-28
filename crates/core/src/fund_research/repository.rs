use async_trait::async_trait;

use crate::errors::Result;

use super::models::{
    FundResearchSnapshot, PortfolioFundPosition, RebalanceAlert, SectorRotationSignal,
    ThemeExposure,
};

#[async_trait]
pub trait FundResearchRepository: Send + Sync {
    async fn upsert_snapshot(&self, snapshot: &FundResearchSnapshot) -> Result<()>;
    async fn latest_snapshot(&self, fund_code: &str) -> Result<Option<FundResearchSnapshot>>;
    async fn snapshots_for_theme(
        &self,
        theme: &str,
        limit: usize,
    ) -> Result<Vec<FundResearchSnapshot>>;
    async fn latest_snapshots_for_funds(
        &self,
        fund_codes: &[String],
    ) -> Result<Vec<FundResearchSnapshot>>;
    async fn upsert_rotation_signals(&self, signals: &[SectorRotationSignal]) -> Result<()>;
    async fn rotation_signals(&self, themes: &[String]) -> Result<Vec<SectorRotationSignal>>;
    async fn upsert_rebalance_alerts(&self, alerts: &[RebalanceAlert]) -> Result<()>;
    async fn rebalance_alerts(&self, fund_code: Option<&str>) -> Result<Vec<RebalanceAlert>>;
}

#[async_trait]
pub trait FundResearchFetcher: Send + Sync {
    async fn fetch_fund_research(&self, fund_code: &str) -> Result<FundResearchSnapshot>;
}

#[async_trait]
pub trait PortfolioFundPositionProvider: Send + Sync {
    async fn fund_positions(&self, portfolio_id: &str) -> Result<Vec<PortfolioFundPosition>>;
}

pub fn exposure_for_theme<'a>(
    exposures: &'a [ThemeExposure],
    theme: &str,
) -> Option<&'a ThemeExposure> {
    exposures
        .iter()
        .find(|exposure| exposure.theme.eq_ignore_ascii_case(theme))
}
