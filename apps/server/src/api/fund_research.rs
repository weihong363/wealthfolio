use std::sync::Arc;

use crate::error::{ApiError, ApiResult};
use crate::main_lib::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use wealthfolio_core::fund_research::{
    FundTopHolding, PortfolioFundLookthroughSummary, StockClassificationOverride,
    UpsertStockClassificationOverride,
};

pub fn fund_research_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/fund-research/funds/{fund_code}/top-holdings",
            get(get_fund_top_holdings),
        )
        .route(
            "/fund-research/portfolios/{portfolio_id}/fund-lookthrough",
            get(get_portfolio_fund_lookthrough),
        )
        .route(
            "/fund-research/funds/{fund_code}/refresh",
            post(refresh_fund_research),
        )
        .route(
            "/fund-research/stock-classifications",
            get(get_stock_classification_overrides).put(save_stock_classification_override),
        )
        .route(
            "/fund-research/stock-classifications/{stock_key}",
            delete(delete_stock_classification_override),
        )
}

async fn get_fund_top_holdings(
    Path(fund_code): Path<String>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<FundTopHolding>>> {
    let holdings = state
        .fund_research_service
        .get_fund_top_holdings(&fund_code)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(holdings))
}

async fn get_portfolio_fund_lookthrough(
    Path(portfolio_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<PortfolioFundLookthroughSummary>> {
    let summary = state
        .fund_research_service
        .analyze_portfolio_fund_lookthrough(&portfolio_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(summary))
}

async fn refresh_fund_research(
    Path(fund_code): Path<String>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<StatusCode> {
    state
        .fund_research_service
        .refresh_fund_research(&fund_code)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(StatusCode::OK)
}

async fn get_stock_classification_overrides(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<StockClassificationOverride>>> {
    let overrides = state
        .fund_research_service
        .get_stock_classification_overrides()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(overrides))
}

async fn save_stock_classification_override(
    State(state): State<Arc<AppState>>,
    Json(input): Json<UpsertStockClassificationOverride>,
) -> ApiResult<Json<StockClassificationOverride>> {
    let override_item = state
        .fund_research_service
        .save_stock_classification_override(input)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(override_item))
}

async fn delete_stock_classification_override(
    Path(stock_key): Path<String>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<StatusCode> {
    state
        .fund_research_service
        .delete_stock_classification_override(&stock_key)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}
