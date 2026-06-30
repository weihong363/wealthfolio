use async_trait::async_trait;
use chrono::NaiveDate;

use crate::errors::Result;

use super::models::ThemeRotationSnapshot;

#[async_trait]
pub trait ThemeRotationRepository: Send + Sync {
    async fn save_theme_rotation_snapshots(
        &self,
        snapshots: &[ThemeRotationSnapshot],
    ) -> Result<()>;
    async fn theme_rotation_snapshots(
        &self,
        theme: Option<&str>,
        since: Option<NaiveDate>,
        limit: usize,
    ) -> Result<Vec<ThemeRotationSnapshot>>;
}
