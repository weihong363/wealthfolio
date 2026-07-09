use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::errors::Result;

use super::models::MarketIntelligenceIntradaySnapshot;

#[async_trait]
pub trait MarketIntelligenceIntradayRepository: Send + Sync {
    async fn save_intraday_snapshots(
        &self,
        snapshots: &[MarketIntelligenceIntradaySnapshot],
    ) -> Result<()>;

    async fn intraday_snapshots(
        &self,
        kind: Option<&str>,
        name: Option<&str>,
        since: Option<DateTime<Utc>>,
        limit: usize,
    ) -> Result<Vec<MarketIntelligenceIntradaySnapshot>>;
}
