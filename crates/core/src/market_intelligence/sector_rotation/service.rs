use std::sync::Arc;

use chrono::NaiveDate;

use crate::errors::Result;

use super::{
    models::SectorRotationSnapshot, provider::SectorRotationProvider,
    repository::SectorRotationRepository,
};

pub struct SectorRotationService {
    repository: Arc<dyn SectorRotationRepository>,
    provider: Option<Arc<dyn SectorRotationProvider>>,
}

impl SectorRotationService {
    pub fn new(repository: Arc<dyn SectorRotationRepository>) -> Self {
        Self {
            repository,
            provider: None,
        }
    }

    pub fn with_provider(mut self, provider: Arc<dyn SectorRotationProvider>) -> Self {
        self.provider = Some(provider);
        self
    }

    pub async fn ingest_snapshots(&self, snapshots: Vec<SectorRotationSnapshot>) -> Result<()> {
        self.repository
            .save_sector_rotation_snapshots(&snapshots)
            .await
    }

    pub async fn refresh_from_provider(
        &self,
        date: Option<NaiveDate>,
    ) -> Result<Vec<SectorRotationSnapshot>> {
        let provider = self.provider.as_ref().ok_or_else(|| {
            crate::errors::Error::Repository(
                "Sector rotation provider is not configured".to_string(),
            )
        })?;
        let snapshots = provider.fetch_sector_rotation_snapshots(date).await?;
        self.repository
            .save_sector_rotation_snapshots(&snapshots)
            .await?;
        Ok(snapshots)
    }

    pub async fn snapshots(
        &self,
        market: Option<&str>,
        sector: Option<&str>,
        since: Option<NaiveDate>,
        limit: usize,
    ) -> Result<Vec<SectorRotationSnapshot>> {
        self.repository
            .sector_rotation_snapshots(market, sector, since, limit)
            .await
    }
}
