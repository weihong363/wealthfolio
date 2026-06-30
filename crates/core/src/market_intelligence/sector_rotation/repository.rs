use async_trait::async_trait;
use chrono::NaiveDate;

use crate::errors::Result;

use super::models::SectorRotationSnapshot;

#[async_trait]
pub trait SectorRotationRepository: Send + Sync {
    async fn save_sector_rotation_snapshots(
        &self,
        snapshots: &[SectorRotationSnapshot],
    ) -> Result<()>;
    async fn sector_rotation_snapshots(
        &self,
        market: Option<&str>,
        sector: Option<&str>,
        since: Option<NaiveDate>,
        limit: usize,
    ) -> Result<Vec<SectorRotationSnapshot>>;
}
