use async_trait::async_trait;

use crate::errors::Result;

use super::models::PortfolioThemeExposure;

#[async_trait]
pub trait PortfolioThemeExposureRepository: Send + Sync {
    async fn save_portfolio_theme_exposures(
        &self,
        exposures: &[PortfolioThemeExposure],
    ) -> Result<()>;
    async fn latest_portfolio_theme_exposures(
        &self,
        portfolio_id: &str,
    ) -> Result<Vec<PortfolioThemeExposure>>;
}
