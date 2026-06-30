use async_trait::async_trait;
use chrono::NaiveDate;

use crate::errors::Result;

use super::models::SectorRotationSnapshot;

#[async_trait]
pub trait SectorRotationProvider: Send + Sync {
    async fn fetch_sector_rotation_snapshots(
        &self,
        date: Option<NaiveDate>,
    ) -> Result<Vec<SectorRotationSnapshot>>;
}
