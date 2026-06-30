use async_trait::async_trait;
use chrono::NaiveDate;

use crate::errors::Result;

use super::models::CapitalFlowSnapshot;

#[async_trait]
pub trait CapitalFlowRepository: Send + Sync {
    async fn save_capital_flow_snapshots(&self, snapshots: &[CapitalFlowSnapshot]) -> Result<()>;
    async fn capital_flow_snapshots(
        &self,
        market: Option<&str>,
        category: Option<&str>,
        since: Option<NaiveDate>,
        limit: usize,
    ) -> Result<Vec<CapitalFlowSnapshot>>;
}
