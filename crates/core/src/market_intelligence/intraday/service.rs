use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::errors::Result;

use super::{
    models::MarketIntelligenceIntradaySnapshot, repository::MarketIntelligenceIntradayRepository,
};

pub struct MarketIntelligenceIntradayService {
    repository: Arc<dyn MarketIntelligenceIntradayRepository>,
}

impl MarketIntelligenceIntradayService {
    pub fn new(repository: Arc<dyn MarketIntelligenceIntradayRepository>) -> Self {
        Self { repository }
    }

    pub async fn ingest_snapshots(
        &self,
        snapshots: Vec<MarketIntelligenceIntradaySnapshot>,
    ) -> Result<()> {
        self.repository.save_intraday_snapshots(&snapshots).await
    }

    pub async fn snapshots(
        &self,
        kind: Option<&str>,
        name: Option<&str>,
        since: Option<DateTime<Utc>>,
        limit: usize,
    ) -> Result<Vec<MarketIntelligenceIntradaySnapshot>> {
        self.repository
            .intraday_snapshots(kind, name, since, limit)
            .await
    }
}
