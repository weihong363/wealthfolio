use async_trait::async_trait;
use chrono::NaiveDate;

use crate::errors::Result;

use super::models::MarketIntelligenceIntradaySnapshot;

#[async_trait]
pub trait IntradayMarketIntelligenceProvider: Send + Sync {
    async fn fetch_intraday_snapshots(
        &self,
        date: Option<NaiveDate>,
    ) -> Result<Vec<MarketIntelligenceIntradaySnapshot>>;
}
