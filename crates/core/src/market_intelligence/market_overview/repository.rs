use async_trait::async_trait;

use crate::errors::Result;

use super::models::MarketSnapshot;

#[async_trait]
pub trait MarketSnapshotRepository: Send + Sync {
    async fn save_market_snapshots(&self, snapshots: &[MarketSnapshot]) -> Result<()>;
    async fn latest_market_snapshots(&self, market: Option<&str>) -> Result<Vec<MarketSnapshot>>;
    async fn market_snapshot_history(
        &self,
        market: &str,
        index_name: &str,
        limit: usize,
    ) -> Result<Vec<MarketSnapshot>>;
}
