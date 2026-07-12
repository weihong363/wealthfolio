use async_trait::async_trait;
use chrono::NaiveDate;

use crate::errors::Result;

use super::models::MacroCapitalSnapshot;

#[async_trait]
pub trait MacroCapitalRepository: Send + Sync {
    async fn save_macro_capital_snapshots(&self, snapshots: &[MacroCapitalSnapshot]) -> Result<()>;
    async fn macro_capital_snapshots(
        &self,
        indicator: Option<&str>,
        market: Option<&str>,
        since: Option<NaiveDate>,
        limit: usize,
    ) -> Result<Vec<MacroCapitalSnapshot>>;
}
