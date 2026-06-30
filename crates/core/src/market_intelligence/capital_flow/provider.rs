use async_trait::async_trait;
use chrono::NaiveDate;

use crate::errors::Result;

use super::models::CapitalFlowSnapshot;

#[async_trait]
pub trait CapitalFlowProvider: Send + Sync {
    async fn fetch_capital_flow_snapshots(
        &self,
        date: Option<NaiveDate>,
    ) -> Result<Vec<CapitalFlowSnapshot>>;
}
