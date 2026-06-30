use std::sync::Arc;

use crate::error::{ApiError, ApiResult};
use crate::main_lib::AppState;
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use reqwest::header;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const EASTMONEY_REFERER: &str = "https://quote.eastmoney.com/center/boardlist.html";
const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36";

#[derive(Deserialize)]
struct ProxyRequest {
    url: String,
    #[serde(default)]
    referer: Option<String>,
}

#[derive(Serialize)]
struct ProxyResponse {
    status: u16,
    body: Value,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/eastmoney/proxy", post(proxy_eastmoney))
}

async fn proxy_eastmoney(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<ProxyRequest>,
) -> ApiResult<Json<ProxyResponse>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent(BROWSER_USER_AGENT)
        .http1_only()
        .build()
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let mut request = client
        .get(&req.url)
        .header(header::ACCEPT, "application/json, text/plain, */*")
        .header(header::ACCEPT_LANGUAGE, "zh-CN,zh;q=0.9,en;q=0.8")
        .header(header::CACHE_CONTROL, "no-cache")
        .header(header::PRAGMA, "no-cache");

    request = request.header(
        header::REFERER,
        req.referer.as_deref().unwrap_or(EASTMONEY_REFERER),
    );

    let response = request
        .send()
        .await
        .map_err(|e| ApiError::Internal(format!("Proxy request failed: {}", e)))?;

    let status = response.status().as_u16();
    let text = response
        .text()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to read response: {}", e)))?;

    let body: Value = serde_json::from_str(&text).unwrap_or(Value::String(text));

    Ok(Json(ProxyResponse { status, body }))
}
