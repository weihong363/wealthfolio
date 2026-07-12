use async_trait::async_trait;
use chrono::NaiveDate;

use crate::errors::Result;

use super::models::MacroCapitalSnapshot;

/// Provider contract for macro capital indicators.
///
/// Implementations must degrade gracefully: an unavailable indicator should be
/// omitted from the returned vector rather than failing the whole fetch, so a
/// single broken upstream endpoint never breaks the dashboard.
#[async_trait]
pub trait MacroCapitalProvider: Send + Sync {
    async fn fetch_macro_capital_snapshots(
        &self,
        date: Option<NaiveDate>,
    ) -> Result<Vec<MacroCapitalSnapshot>>;
}
