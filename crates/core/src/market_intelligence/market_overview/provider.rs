use async_trait::async_trait;

use crate::errors::Result;

use super::models::MarketSnapshot;

#[async_trait]
pub trait MarketDataProvider: Send + Sync {
    async fn fetch_market_snapshots(&self) -> Result<Vec<MarketSnapshot>>;
}
