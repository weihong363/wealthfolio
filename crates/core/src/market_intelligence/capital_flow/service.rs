use std::sync::Arc;

use chrono::NaiveDate;

use crate::errors::Result;

use super::{
    models::CapitalFlowSnapshot, provider::CapitalFlowProvider, repository::CapitalFlowRepository,
};

pub struct CapitalFlowService {
    repository: Arc<dyn CapitalFlowRepository>,
    provider: Option<Arc<dyn CapitalFlowProvider>>,
}

impl CapitalFlowService {
    pub fn new(repository: Arc<dyn CapitalFlowRepository>) -> Self {
        Self {
            repository,
            provider: None,
        }
    }

    pub fn with_provider(mut self, provider: Arc<dyn CapitalFlowProvider>) -> Self {
        self.provider = Some(provider);
        self
    }

    pub async fn ingest_snapshots(&self, snapshots: Vec<CapitalFlowSnapshot>) -> Result<()> {
        self.repository
            .save_capital_flow_snapshots(&snapshots)
            .await
    }

    pub async fn refresh_from_provider(
        &self,
        date: Option<NaiveDate>,
    ) -> Result<Vec<CapitalFlowSnapshot>> {
        let provider = self.provider.as_ref().ok_or_else(|| {
            crate::errors::Error::Repository("Capital flow provider is not configured".to_string())
        })?;
        let snapshots = provider.fetch_capital_flow_snapshots(date).await?;
        self.repository
            .save_capital_flow_snapshots(&snapshots)
            .await?;
        Ok(snapshots)
    }

    pub async fn snapshots(
        &self,
        market: Option<&str>,
        category: Option<&str>,
        since: Option<NaiveDate>,
        limit: usize,
    ) -> Result<Vec<CapitalFlowSnapshot>> {
        self.repository
            .capital_flow_snapshots(market, category, since, limit)
            .await
    }
}
