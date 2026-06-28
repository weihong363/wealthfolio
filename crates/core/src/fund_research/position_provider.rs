use std::sync::Arc;

use async_trait::async_trait;
use num_traits::ToPrimitive;

use crate::accounts::AccountServiceTrait;
use crate::assets::AssetKind;
use crate::errors::Result;
use crate::portfolio::holdings::holdings_service::HoldingsServiceTrait;

use super::models::PortfolioFundPosition;
use super::repository::PortfolioFundPositionProvider;

/// Provides fund positions by querying the holdings service for all active
/// accounts and filtering to Investment-kind assets (funds, ETFs, stocks).
pub struct HoldingsBasedPositionProvider {
    account_service: Arc<dyn AccountServiceTrait>,
    holdings_service: Arc<dyn HoldingsServiceTrait>,
    base_currency: String,
}

impl HoldingsBasedPositionProvider {
    pub fn new(
        account_service: Arc<dyn AccountServiceTrait>,
        holdings_service: Arc<dyn HoldingsServiceTrait>,
        base_currency: String,
    ) -> Self {
        Self {
            account_service,
            holdings_service,
            base_currency,
        }
    }
}

#[async_trait]
impl PortfolioFundPositionProvider for HoldingsBasedPositionProvider {
    async fn fund_positions(&self, portfolio_id: &str) -> Result<Vec<PortfolioFundPosition>> {
        let accounts = self.account_service.get_active_accounts()?;
        if accounts.is_empty() {
            return Ok(Vec::new());
        }

        let account_ids: Vec<String> = accounts.iter().map(|a| a.id.clone()).collect();
        let holdings = self
            .holdings_service
            .get_holdings_for_accounts(&account_ids, &self.base_currency, portfolio_id)
            .await?;

        let positions: Vec<PortfolioFundPosition> = holdings
            .into_iter()
            .filter(|h| h.asset_kind == Some(AssetKind::Investment))
            .filter_map(|h| {
                let instrument = h.instrument.as_ref()?;
                let market_value_base = h.market_value.base.to_f64().unwrap_or(0.0);
                if market_value_base <= 0.0 {
                    return None;
                }
                Some(PortfolioFundPosition {
                    portfolio_id: portfolio_id.to_string(),
                    fund_code: instrument.symbol.clone(),
                    market_value_base,
                })
            })
            .collect();

        Ok(positions)
    }
}
