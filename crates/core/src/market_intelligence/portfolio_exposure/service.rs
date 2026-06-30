use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::Utc;

use crate::errors::Result;

use super::{
    models::{FundThemeExposureInput, PortfolioThemeExposure},
    provider::PortfolioExposureInputProvider,
    repository::PortfolioThemeExposureRepository,
};

pub struct PortfolioExposureService {
    repository: Arc<dyn PortfolioThemeExposureRepository>,
    input_provider: Option<Arc<dyn PortfolioExposureInputProvider>>,
}

impl PortfolioExposureService {
    pub fn new(repository: Arc<dyn PortfolioThemeExposureRepository>) -> Self {
        Self {
            repository,
            input_provider: None,
        }
    }

    pub fn with_input_provider(
        mut self,
        provider: Arc<dyn PortfolioExposureInputProvider>,
    ) -> Self {
        self.input_provider = Some(provider);
        self
    }

    pub async fn calculate_and_save(
        &self,
        portfolio_id: &str,
        inputs: &[FundThemeExposureInput],
    ) -> Result<Vec<PortfolioThemeExposure>> {
        let exposures = calculate_portfolio_theme_exposure(portfolio_id, inputs);
        self.repository
            .save_portfolio_theme_exposures(&exposures)
            .await?;
        Ok(exposures)
    }

    pub async fn calculate_from_provider(
        &self,
        portfolio_id: &str,
    ) -> Result<Vec<PortfolioThemeExposure>> {
        let provider = self.input_provider.as_ref().ok_or_else(|| {
            crate::errors::Error::Repository(
                "Portfolio exposure input provider is not configured".to_string(),
            )
        })?;
        let inputs = provider.fund_theme_exposures(portfolio_id).await?;
        self.calculate_and_save(portfolio_id, &inputs).await
    }

    pub async fn latest(&self, portfolio_id: &str) -> Result<Vec<PortfolioThemeExposure>> {
        self.repository
            .latest_portfolio_theme_exposures(portfolio_id)
            .await
    }

    pub async fn save_exposures(&self, exposures: &[PortfolioThemeExposure]) -> Result<()> {
        self.repository
            .save_portfolio_theme_exposures(exposures)
            .await
    }
}

pub fn calculate_portfolio_theme_exposure(
    portfolio_id: &str,
    inputs: &[FundThemeExposureInput],
) -> Vec<PortfolioThemeExposure> {
    let total_market_value: f64 = inputs
        .iter()
        .map(|input| input.fund_market_value * input.theme_weight_pct / 100.0)
        .sum();
    let mut grouped: BTreeMap<String, (f64, String)> = BTreeMap::new();

    for input in inputs {
        let theme_value = input.fund_market_value * input.theme_weight_pct / 100.0;
        let entry = grouped
            .entry(input.theme.clone())
            .or_insert_with(|| (0.0, input.source.clone()));
        entry.0 += theme_value;
    }

    let now = Utc::now();
    grouped
        .into_iter()
        .map(|(theme, (market_value, source))| PortfolioThemeExposure {
            portfolio_id: portfolio_id.to_string(),
            theme,
            weight_pct: percentage(market_value, total_market_value),
            market_value,
            source,
            timestamp: now,
        })
        .collect()
}

fn percentage(value: f64, total: f64) -> f64 {
    if total.abs() < f64::EPSILON {
        return 0.0;
    }
    value / total * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_portfolio_theme_exposure_from_fund_inputs() {
        let inputs = vec![
            input("000001", 1000.0, "AI", 50.0),
            input("000002", 500.0, "AI", 20.0),
            input("000002", 500.0, "Cash", 10.0),
        ];

        let result = calculate_portfolio_theme_exposure("p1", &inputs);
        let ai = result.iter().find(|item| item.theme == "AI").unwrap();

        assert_eq!(ai.market_value, 600.0);
        assert!((ai.weight_pct - 92.307692).abs() < 0.0001);
    }

    fn input(
        fund_code: &str,
        fund_market_value: f64,
        theme: &str,
        theme_weight_pct: f64,
    ) -> FundThemeExposureInput {
        FundThemeExposureInput {
            fund_code: fund_code.to_string(),
            fund_weight_pct: 0.0,
            fund_market_value,
            theme: theme.to_string(),
            theme_weight_pct,
            source: "test".to_string(),
        }
    }
}
