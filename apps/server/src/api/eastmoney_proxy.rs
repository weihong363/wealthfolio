use std::sync::Arc;
use std::sync::LazyLock;

use crate::error::{ApiError, ApiResult};
use crate::main_lib::AppState;
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use reqwest::header;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const EASTMONEY_REFERER: &str = "https://quote.eastmoney.com/center/boardlist.html";
const EASTMONEY_ORIGIN: &str = "https://quote.eastmoney.com";
const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36";
const MAX_RETRIES: u32 = 2;

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    // Mirror the browser transport used by wealthfolio-core's Eastmoney client:
    // automatic decompression, a cookie jar and keep-alive. Eastmoney's WAF
    // resets connections whose fingerprint deviates from a real browser, which
    // surfaces here as "error sending request for url".
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent(BROWSER_USER_AGENT)
        .http1_only()
        .cookie_store(true)
        .gzip(true)
        .brotli(true)
        .deflate(true)
        .tcp_nodelay(true)
        .tcp_keepalive(std::time::Duration::from_secs(60))
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .build()
        .expect("failed to build eastmoney proxy client")
});

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
    let referer = req.referer.as_deref().unwrap_or(EASTMONEY_REFERER);
    let mut last_err = String::new();

    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(300 * attempt as u64)).await;
        }

        let request = CLIENT
            .get(&req.url)
            .header(header::ACCEPT, "application/json, text/plain, */*")
            .header(header::ACCEPT_LANGUAGE, "zh-CN,zh;q=0.9,en;q=0.8")
            .header(header::REFERER, referer)
            .header(header::ORIGIN, EASTMONEY_ORIGIN)
            .header(header::CACHE_CONTROL, "no-cache")
            .header(header::PRAGMA, "no-cache");

        match request.send().await {
            Ok(response) => {
                let status = response.status().as_u16();
                let text = response
                    .text()
                    .await
                    .map_err(|e| ApiError::Internal(format!("Failed to read response: {}", e)))?;
                let body: Value = serde_json::from_str(&text).unwrap_or(Value::String(text));
                return Ok(Json(ProxyResponse { status, body }));
            }
            Err(e) => {
                last_err = e.to_string();
                tracing::warn!(
                    "EastMoney proxy attempt {} failed for {}: {}",
                    attempt + 1,
                    req.url,
                    last_err
                );
            }
        }
    }

    Err(ApiError::Internal(format!(
        "Proxy request failed after {} retries: {}",
        MAX_RETRIES + 1,
        last_err
    )))
}
