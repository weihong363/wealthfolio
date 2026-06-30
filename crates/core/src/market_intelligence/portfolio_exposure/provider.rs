use async_trait::async_trait;

use crate::errors::Result;

use super::models::FundThemeExposureInput;

#[async_trait]
pub trait PortfolioExposureInputProvider: Send + Sync {
    async fn fund_theme_exposures(&self, portfolio_id: &str)
        -> Result<Vec<FundThemeExposureInput>>;
}
