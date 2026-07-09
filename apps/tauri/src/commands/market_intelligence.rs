use std::sync::Arc;

use crate::context::ServiceContext;
use serde::Deserialize;
use tauri::State;
use wealthfolio_core::market_intelligence::{
    CapitalFlowSnapshot, MarketIntelligenceSummary, MarketSnapshot, SectorRotationSnapshot,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserSnapshots {
    market_overview: Vec<MarketSnapshot>,
    capital_flow: Vec<CapitalFlowSnapshot>,
    sector_rotation: Vec<SectorRotationSnapshot>,
}

#[tauri::command]
pub async fn get_market_intelligence_summary(
    portfolio_id: Option<String>,
    window: Option<String>,
    context: State<'_, Arc<ServiceContext>>,
) -> Result<MarketIntelligenceSummary, String> {
    context
        .market_intelligence_service
        .summary_for_window(portfolio_id.as_deref(), window.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn refresh_market_intelligence(
    context: State<'_, Arc<ServiceContext>>,
) -> Result<MarketIntelligenceSummary, String> {
    context
        .market_intelligence_service
        .refresh_market_data()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ingest_market_intelligence_snapshots(
    snapshots: BrowserSnapshots,
    context: State<'_, Arc<ServiceContext>>,
) -> Result<MarketIntelligenceSummary, String> {
    context
        .market_intelligence_service
        .ingest_snapshots(
            snapshots.market_overview,
            snapshots.capital_flow,
            snapshots.sector_rotation,
        )
        .await
        .map_err(|e| e.to_string())
}
