use std::sync::Arc;

use crate::errors::Result;

use super::{
    models::MarketSnapshot, provider::MarketDataProvider, repository::MarketSnapshotRepository,
};

pub struct MarketOverviewService {
    repository: Arc<dyn MarketSnapshotRepository>,
    provider: Option<Arc<dyn MarketDataProvider>>,
}

impl MarketOverviewService {
    pub fn new(repository: Arc<dyn MarketSnapshotRepository>) -> Self {
        Self {
            repository,
            provider: None,
        }
    }

    pub fn with_provider(mut self, provider: Arc<dyn MarketDataProvider>) -> Self {
        self.provider = Some(provider);
        self
    }

    pub async fn ingest_snapshots(&self, snapshots: Vec<MarketSnapshot>) -> Result<()> {
        self.repository.save_market_snapshots(&snapshots).await
    }

    pub async fn refresh_from_provider(&self) -> Result<Vec<MarketSnapshot>> {
        let provider = self.provider.as_ref().ok_or_else(|| {
            crate::errors::Error::Repository("Market data provider is not configured".to_string())
        })?;
        let snapshots = provider.fetch_market_snapshots().await?;
        self.repository.save_market_snapshots(&snapshots).await?;
        Ok(snapshots)
    }

    pub async fn latest(&self, market: Option<&str>) -> Result<Vec<MarketSnapshot>> {
        self.repository.latest_market_snapshots(market).await
    }

    /// Historical snapshots for a single index (newest first), used e.g. to
    /// resolve the previous trading day's turnover for flow-signal analysis.
    pub async fn history(
        &self,
        market: &str,
        index_name: &str,
        limit: usize,
    ) -> Result<Vec<MarketSnapshot>> {
        self.repository
            .market_snapshot_history(market, index_name, limit)
            .await
    }
}
