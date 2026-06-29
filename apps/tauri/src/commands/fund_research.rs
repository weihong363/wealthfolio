use std::sync::Arc;

use crate::context::ServiceContext;
use tauri::State;
use wealthfolio_core::fund_research::{
    FundTopHolding, PortfolioFundLookthroughSummary, StockClassificationOverride,
    UpsertStockClassificationOverride,
};

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

#[tauri::command]
pub async fn get_stock_classification_overrides(
    context: State<'_, Arc<ServiceContext>>,
) -> Result<Vec<StockClassificationOverride>, String> {
    context
        .fund_research_service
        .get_stock_classification_overrides()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_stock_classification_override(
    input: UpsertStockClassificationOverride,
    context: State<'_, Arc<ServiceContext>>,
) -> Result<StockClassificationOverride, String> {
    context
        .fund_research_service
        .save_stock_classification_override(input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_stock_classification_override(
    stock_key: String,
    context: State<'_, Arc<ServiceContext>>,
) -> Result<(), String> {
    context
        .fund_research_service
        .delete_stock_classification_override(&stock_key)
        .await
        .map_err(|e| e.to_string())
}
