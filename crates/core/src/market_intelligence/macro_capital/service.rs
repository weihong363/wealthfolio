use std::sync::Arc;

use chrono::NaiveDate;

use crate::errors::Result;

use super::{
    models::MacroCapitalSnapshot, provider::MacroCapitalProvider,
    repository::MacroCapitalRepository,
};

/// Service for macro capital indicators.
///
/// Mirrors [`CapitalFlowService`](super::super::capital_flow::CapitalFlowService):
/// a repository is always present, the provider is optional and injected via a
/// builder. The orchestrating [`MarketIntelligenceService`] owns provider calls,
/// so keeping the provider optional keeps this sub-service usable for pure
/// persistence/query flows too.
pub struct MacroCapitalService {
    repository: Arc<dyn MacroCapitalRepository>,
    provider: Option<Arc<dyn MacroCapitalProvider>>,
}

impl MacroCapitalService {
    pub fn new(repository: Arc<dyn MacroCapitalRepository>) -> Self {
        Self {
            repository,
            provider: None,
        }
    }

    pub fn with_provider(mut self, provider: Arc<dyn MacroCapitalProvider>) -> Self {
        self.provider = Some(provider);
        self
    }

    pub async fn ingest_snapshots(&self, snapshots: Vec<MacroCapitalSnapshot>) -> Result<()> {
        self.repository
            .save_macro_capital_snapshots(&snapshots)
            .await
    }

    pub async fn refresh_from_provider(
        &self,
        date: Option<NaiveDate>,
    ) -> Result<Vec<MacroCapitalSnapshot>> {
        let provider = self.provider.as_ref().ok_or_else(|| {
            crate::errors::Error::Repository("Macro capital provider is not configured".to_string())
        })?;
        let snapshots = provider.fetch_macro_capital_snapshots(date).await?;
        self.repository
            .save_macro_capital_snapshots(&snapshots)
            .await?;
        Ok(snapshots)
    }

    pub async fn snapshots(
        &self,
        indicator: Option<&str>,
        market: Option<&str>,
        since: Option<NaiveDate>,
        limit: usize,
    ) -> Result<Vec<MacroCapitalSnapshot>> {
        self.repository
            .macro_capital_snapshots(indicator, market, since, limit)
            .await
    }
}
