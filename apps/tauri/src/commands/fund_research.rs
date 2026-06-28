use std::sync::Arc;

use crate::context::ServiceContext;
use tauri::State;
use wealthfolio_core::fund_research::{FundTopHolding, PortfolioFundLookthroughSummary};

#[tauri::command]
pub async fn get_fund_top_holdings(
    fund_code: String,
    context: State<'_, Arc<ServiceContext>>,
) -> Result<Vec<FundTopHolding>, String> {
    context
        .fund_research_service
        .get_fund_top_holdings(&fund_code)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_portfolio_fund_lookthrough(
    portfolio_id: String,
    context: State<'_, Arc<ServiceContext>>,
) -> Result<PortfolioFundLookthroughSummary, String> {
    context
        .fund_research_service
        .analyze_portfolio_fund_lookthrough(&portfolio_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn refresh_fund_research(
    fund_code: String,
    context: State<'_, Arc<ServiceContext>>,
) -> Result<wealthfolio_core::fund_research::FundResearchSnapshot, String> {
    context
        .fund_research_service
        .refresh_fund_research(&fund_code)
        .await
        .map_err(|e| e.to_string())
}
