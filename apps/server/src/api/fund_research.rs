use std::sync::Arc;

use crate::error::{ApiError, ApiResult};
use crate::main_lib::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use wealthfolio_core::fund_research::{FundTopHolding, PortfolioFundLookthroughSummary};

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
