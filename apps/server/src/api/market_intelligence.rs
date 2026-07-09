use std::sync::Arc;

use crate::error::{ApiError, ApiResult};
use crate::main_lib::AppState;
use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use wealthfolio_core::market_intelligence::{
    CapitalFlowSnapshot, MarketIntelligenceSummary, MarketSnapshot, SectorRotationSnapshot,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MarketIntelligenceQuery {
    portfolio_id: Option<String>,
    window: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BrowserSnapshotRequest {
    snapshots: BrowserSnapshots,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BrowserSnapshots {
    market_overview: Vec<MarketSnapshot>,
    capital_flow: Vec<CapitalFlowSnapshot>,
    sector_rotation: Vec<SectorRotationSnapshot>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/market-intelligence/summary", get(get_summary))
        .route("/market-intelligence/refresh", post(refresh))
        .route(
            "/market-intelligence/browser-snapshots",
            post(ingest_browser_snapshots),
        )
}

async fn get_summary(
    State(state): State<Arc<AppState>>,
    Query(query): Query<MarketIntelligenceQuery>,
) -> ApiResult<Json<MarketIntelligenceSummary>> {
    let summary = state
        .market_intelligence_service
        .summary_for_window(query.portfolio_id.as_deref(), query.window.as_deref())
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(summary))
}

async fn refresh(State(state): State<Arc<AppState>>) -> ApiResult<Json<MarketIntelligenceSummary>> {
    let summary = state
        .market_intelligence_service
        .refresh_market_data()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(summary))
}

async fn ingest_browser_snapshots(
    State(state): State<Arc<AppState>>,
    Json(request): Json<BrowserSnapshotRequest>,
) -> ApiResult<Json<MarketIntelligenceSummary>> {
    let summary = state
        .market_intelligence_service
        .ingest_snapshots(
            request.snapshots.market_overview,
            request.snapshots.capital_flow,
            request.snapshots.sector_rotation,
        )
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(summary))
}
