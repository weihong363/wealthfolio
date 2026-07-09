use std::sync::Arc;

use chrono::NaiveDate;

use crate::errors::Result;
use crate::market_intelligence::sector_rotation::SectorRotationSnapshot;

use super::{
    models::{ThemeMapping, ThemeRotationRules, ThemeRotationSnapshot},
    provider::ThemeMappingProvider,
    repository::ThemeRotationRepository,
};

pub struct ThemeRotationService {
    repository: Arc<dyn ThemeRotationRepository>,
    mapping_provider: Option<Arc<dyn ThemeMappingProvider>>,
}

impl ThemeRotationService {
    pub fn new(repository: Arc<dyn ThemeRotationRepository>) -> Self {
        Self {
            repository,
            mapping_provider: None,
        }
    }

    pub fn with_mapping_provider(mut self, provider: Arc<dyn ThemeMappingProvider>) -> Self {
        self.mapping_provider = Some(provider);
        self
    }

    pub async fn ingest_snapshots(&self, snapshots: Vec<ThemeRotationSnapshot>) -> Result<()> {
        self.repository
            .save_theme_rotation_snapshots(&snapshots)
            .await
    }

    pub async fn calculate_and_save(
        &self,
        mappings: Vec<ThemeMapping>,
        sectors: &[SectorRotationSnapshot],
    ) -> Result<Vec<ThemeRotationSnapshot>> {
        let snapshots = ThemeRotationRules::new(mappings).calculate(sectors);
        self.repository
            .save_theme_rotation_snapshots(&snapshots)
            .await?;
        Ok(snapshots)
    }

    pub async fn calculate_from_provider(
        &self,
        sectors: &[SectorRotationSnapshot],
    ) -> Result<Vec<ThemeRotationSnapshot>> {
        let provider = self.mapping_provider.as_ref().ok_or_else(|| {
            crate::errors::Error::Repository("Theme mapping provider is not configured".to_string())
        })?;
        self.calculate_and_save(provider.theme_mappings().await?, sectors)
            .await
    }

    pub async fn snapshots(
        &self,
        theme: Option<&str>,
        since: Option<NaiveDate>,
        limit: usize,
    ) -> Result<Vec<ThemeRotationSnapshot>> {
        self.repository
            .theme_rotation_snapshots(theme, since, limit)
            .await
    }
}
