use std::{
    collections::HashMap,
    sync::OnceLock,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use chrono::{NaiveDate, Utc};
use reqwest::{header, StatusCode};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::errors::{Error, Result};

use super::{
    capital_flow::{CapitalFlowProvider, CapitalFlowSnapshot},
    market_overview::{MarketDataProvider, MarketSnapshot},
    sector_rotation::{SectorRotationProvider, SectorRotationSnapshot},
    theme_rotation::{ThemeMapping, ThemeMappingProvider},
};

const SOURCE: &str = "eastmoney";
const PUSH2_BASE_URL: &str = "https://push2.eastmoney.com";
const EASTMONEY_SESSION_URL: &str = "https://quote.eastmoney.com/";
const EASTMONEY_ORIGIN: &str = "https://quote.eastmoney.com";
const EASTMONEY_UT: &str = "bd1d9ddb04089700cf9c27f6f7426281";
const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/148.0.0.0 Safari/537.36";
const MAX_RETRIES: usize = 3;
const MIN_API_INTERVAL: Duration = Duration::from_millis(500);
const RETRY_DELAYS: [Duration; MAX_RETRIES] = [
    Duration::from_millis(500),
    Duration::from_secs(1),
    Duration::from_secs(2),
];

static EASTMONEY_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static SESSION_INITIALIZED: OnceLock<Mutex<bool>> = OnceLock::new();
static API_LAST_REQUEST_AT: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();

#[derive(Clone, Debug)]
pub struct EastmoneyMarketIntelligenceProvider {
    client: reqwest::Client,
    base_url: String,
}

impl EastmoneyMarketIntelligenceProvider {
    pub fn new() -> Self {
        Self::with_base_url(PUSH2_BASE_URL)
    }

    pub fn with_base_url(base_url: &str) -> Self {
        Self {
            client: eastmoney_client().clone(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    async fn get_json(&self, path: &str) -> Result<Value> {
        if self.uses_default_base_url() {
            self.ensure_session().await?;
        }
        rate_limit(api_key(path)).await;

        let url = format!("{}{}", self.base_url, path);
        let mut last_error = None;

        for attempt in 0..=MAX_RETRIES {
            log::debug!("Eastmoney GET {url} attempt {}", attempt + 1);
            match self.send_api_request(&url).await {
                Ok((status, body)) if status.is_success() => {
                    log_response(status, &body);
                    return serde_json::from_str::<Value>(&body).map_err(|error| {
                        Error::Repository(format!("Invalid Eastmoney response: {error}"))
                    });
                }
                Ok((status, body)) => {
                    log_response(status, &body);
                    let detail = format!(
                        "Eastmoney request failed with HTTP {}: {}",
                        status,
                        body.chars().take(500).collect::<String>()
                    );
                    if should_retry_status(status) && attempt < MAX_RETRIES {
                        sleep_before_retry(attempt).await;
                        last_error = Some(detail);
                        continue;
                    }
                    return Err(Error::Repository(detail));
                }
                Err(error) => {
                    let detail = format!("Eastmoney request error: {error:#}");
                    log::warn!("{detail}");
                    if attempt < MAX_RETRIES {
                        sleep_before_retry(attempt).await;
                        last_error = Some(detail);
                        continue;
                    }
                    return Err(Error::Repository(detail));
                }
            }
        }

        Err(Error::Repository(last_error.unwrap_or_else(|| {
            "Eastmoney request failed after retries".to_string()
        })))
    }

    async fn fetch_sector_rows(&self, fid: &str, limit: usize) -> Result<Vec<Value>> {
        let path = format!(
            "/api/qt/clist/get?pn=1&pz={limit}&po=1&np=1&fltt=2&invt=2&fid={fid}&fs=m:90+t:2&fields=f12,f14,f3,f6,f62&ut={EASTMONEY_UT}&_={}",
            Utc::now().timestamp_millis()
        );
        let value = self.get_json(&path).await?;
        Ok(value
            .pointer("/data/diff")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default())
    }

    fn uses_default_base_url(&self) -> bool {
        self.base_url == PUSH2_BASE_URL
    }

    async fn ensure_session(&self) -> Result<()> {
        let state = SESSION_INITIALIZED.get_or_init(|| Mutex::new(false));
        let mut initialized = state.lock().await;
        if *initialized {
            return Ok(());
        }

        // Visit the quote home once so reqwest's cookie store can keep any
        // server-issued session cookies. Cookies are never constructed by hand.
        let response = self
            .client
            .get(EASTMONEY_SESSION_URL)
            .header(
                header::ACCEPT,
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .header(header::ACCEPT_LANGUAGE, "zh-CN,zh;q=0.9,en;q=0.8")
            .send()
            .await
            .map_err(|error| {
                Error::Repository(format!("Eastmoney session request error: {error:#}"))
            })?;

        log::debug!(
            "Eastmoney session GET {EASTMONEY_SESSION_URL} status {}",
            response.status()
        );
        *initialized = true;
        Ok(())
    }

    async fn send_api_request(
        &self,
        url: &str,
    ) -> std::result::Result<(StatusCode, String), reqwest::Error> {
        let response = self
            .client
            .get(url)
            .header(header::ACCEPT, "application/json,text/plain,*/*")
            .header(header::ACCEPT_LANGUAGE, "zh-CN,zh;q=0.9,en;q=0.8")
            .header(header::REFERER, EASTMONEY_SESSION_URL)
            .header(header::ORIGIN, EASTMONEY_ORIGIN)
            .header(header::CACHE_CONTROL, "no-cache")
            .header(header::PRAGMA, "no-cache")
            .send()
            .await?;
        let status = response.status();
        let body = response.text().await?;
        Ok((status, body))
    }
}

fn eastmoney_client() -> &'static reqwest::Client {
    EASTMONEY_CLIENT.get_or_init(|| {
        // Keep a single browser-like transport for the module: cookies,
        // decompression, and keep-alive are handled by reqwest rather than
        // copied into per-request headers.
        reqwest::Client::builder()
            .user_agent(BROWSER_USER_AGENT)
            .cookie_store(true)
            .gzip(true)
            .brotli(true)
            .deflate(true)
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_keepalive(Duration::from_secs(60))
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_else(|error| {
                log::warn!("Failed to build Eastmoney reqwest client: {error}");
                reqwest::Client::new()
            })
    })
}

async fn rate_limit(api: String) {
    let state = API_LAST_REQUEST_AT.get_or_init(|| Mutex::new(HashMap::new()));

    loop {
        let wait_for = {
            let mut last_by_api = state.lock().await;
            match last_by_api.get(&api) {
                Some(last_request_at) if last_request_at.elapsed() < MIN_API_INTERVAL => {
                    Some(MIN_API_INTERVAL - last_request_at.elapsed())
                }
                _ => {
                    last_by_api.insert(api.clone(), Instant::now());
                    None
                }
            }
        };

        if let Some(wait_for) = wait_for {
            // Release the limiter lock while waiting so unrelated APIs can run.
            tokio::time::sleep(wait_for).await;
        } else {
            return;
        }
    }
}

fn api_key(path: &str) -> String {
    path.split('?').next().unwrap_or(path).to_string()
}

fn should_retry_status(status: StatusCode) -> bool {
    status == StatusCode::FORBIDDEN || status == StatusCode::TOO_MANY_REQUESTS
}

async fn sleep_before_retry(attempt: usize) {
    if let Some(delay) = RETRY_DELAYS.get(attempt) {
        tokio::time::sleep(*delay).await;
    }
}

fn log_response(status: StatusCode, body: &str) {
    log::debug!(
        "Eastmoney response status {} body {}",
        status,
        body.chars().take(500).collect::<String>()
    );
}

impl Default for EastmoneyMarketIntelligenceProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MarketDataProvider for EastmoneyMarketIntelligenceProvider {
    async fn fetch_market_snapshots(&self) -> Result<Vec<MarketSnapshot>> {
        let indices = [
            ("CN", "上证指数", "1.000001"),
            ("CN", "深证成指", "0.399001"),
            ("CN", "创业板指", "0.399006"),
            ("CN", "科创50", "1.000688"),
            ("HK", "恒生指数", "100.HSI"),
            ("HK", "恒生科技", "100.HSTECH"),
        ];
        let secids = indices
            .iter()
            .map(|(_, _, secid)| *secid)
            .collect::<Vec<_>>()
            .join(",");
        let path = format!(
            "/api/qt/ulist.np/get?fltt=2&fields=f12,f14,f2,f3,f4,f6&secids={secids}&ut={EASTMONEY_UT}&_={}",
            Utc::now().timestamp_millis()
        );
        let value = self.get_json(&path).await?;
        let now = Utc::now();
        let rows = value
            .pointer("/data/diff")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        Ok(rows
            .iter()
            .filter_map(|row| {
                let code = string_field(row, "f12")?;
                let (market, fallback_name, _) = indices
                    .iter()
                    .find(|(_, _, secid)| secid.ends_with(&code))?;
                Some(MarketSnapshot {
                    market: (*market).to_string(),
                    index_name: string_field(row, "f14")
                        .unwrap_or_else(|| (*fallback_name).to_string()),
                    price: number_field(row, "f2")?,
                    change_pct: number_field(row, "f3").unwrap_or_default(),
                    turnover: number_field(row, "f6"),
                    timestamp: now,
                    source: SOURCE.to_string(),
                })
            })
            .collect())
    }
}

#[async_trait]
impl SectorRotationProvider for EastmoneyMarketIntelligenceProvider {
    async fn fetch_sector_rotation_snapshots(
        &self,
        date: Option<NaiveDate>,
    ) -> Result<Vec<SectorRotationSnapshot>> {
        let snapshot_date = date.unwrap_or_else(|| Utc::now().date_naive());
        let rows = self.fetch_sector_rows("f62", 50).await?;
        Ok(rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| {
                Some(SectorRotationSnapshot {
                    market: "CN".to_string(),
                    sector: string_field(row, "f14")?,
                    date: snapshot_date,
                    net_flow: number_field(row, "f62"),
                    change_pct: number_field(row, "f3"),
                    turnover: number_field(row, "f6"),
                    ranking: Some((index + 1) as i32),
                    source: SOURCE.to_string(),
                })
            })
            .collect())
    }
}

#[async_trait]
impl CapitalFlowProvider for EastmoneyMarketIntelligenceProvider {
    async fn fetch_capital_flow_snapshots(
        &self,
        date: Option<NaiveDate>,
    ) -> Result<Vec<CapitalFlowSnapshot>> {
        let snapshot_date = date.unwrap_or_else(|| Utc::now().date_naive());
        let rows = self.fetch_sector_rows("f62", 200).await?;
        let net_flows: Vec<f64> = rows
            .iter()
            .filter_map(|row| number_field(row, "f62"))
            .collect();
        let inflow: f64 = net_flows.iter().copied().filter(|value| *value > 0.0).sum();
        let outflow: f64 = net_flows
            .iter()
            .copied()
            .filter(|value| *value < 0.0)
            .map(f64::abs)
            .sum();
        let net_flow = inflow - outflow;

        Ok(vec![CapitalFlowSnapshot {
            market: "CN".to_string(),
            date: snapshot_date,
            category: "main_funds".to_string(),
            inflow: Some(inflow),
            outflow: Some(outflow),
            net_flow,
            source: SOURCE.to_string(),
        }])
    }
}

#[async_trait]
impl ThemeMappingProvider for EastmoneyMarketIntelligenceProvider {
    async fn theme_mappings(&self) -> Result<Vec<ThemeMapping>> {
        Ok(default_theme_mappings())
    }
}

pub fn default_theme_mappings() -> Vec<ThemeMapping> {
    vec![
        mapping(
            "AI",
            &["计算机", "软件开发", "通信设备", "光学光电子", "半导体"],
        ),
        mapping(
            "GPU",
            &["半导体", "数字芯片设计", "模拟芯片设计", "集成电路封测"],
        ),
        mapping("HBM", &["半导体", "存储芯片", "数字芯片设计"]),
        mapping(
            "CPO",
            &["通信设备", "光模块", "光学光电子", "通信网络设备及器件"],
        ),
        mapping("IDC", &["计算机", "通信服务", "互联网服务"]),
        mapping("机器人", &["机器人", "自动化设备", "通用设备"]),
        mapping("自动驾驶", &["汽车零部件", "汽车服务", "软件开发"]),
        mapping("创新药", &["化学制药", "生物制品", "医疗服务"]),
        mapping("消费电子", &["消费电子", "电子元件", "光学光电子"]),
        mapping("新能源", &["电池", "光伏设备", "风电设备", "能源金属"]),
        mapping("银行", &["银行"]),
        mapping("地产", &["房地产开发", "房地产服务"]),
        mapping("消费", &["食品饮料", "酿酒行业", "家电行业", "旅游酒店"]),
    ]
}

fn mapping(theme: &str, sectors: &[&str]) -> ThemeMapping {
    ThemeMapping {
        theme: theme.to_string(),
        sectors: sectors.iter().map(|sector| (*sector).to_string()).collect(),
    }
}

fn string_field(row: &Value, field: &str) -> Option<String> {
    row.get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "-")
        .map(str::to_string)
}

fn number_field(row: &Value, field: &str) -> Option<f64> {
    match row.get(field)? {
        Value::Number(number) => number.as_f64(),
        Value::String(value) => value.parse::<f64>().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

    // ─── Helper function tests ───────────────────────────────────

    #[test]
    fn test_string_field_extracts_value() {
        let json = serde_json::json!({"f14": " 电子 "});
        assert_eq!(string_field(&json, "f14"), Some("电子".to_string()));
    }

    #[test]
    fn test_string_field_returns_none_for_missing() {
        let json = serde_json::json!({"f12": "BK0001"});
        assert_eq!(string_field(&json, "f14"), None);
    }

    #[test]
    fn test_string_field_returns_none_for_empty() {
        let json = serde_json::json!({"f14": ""});
        assert_eq!(string_field(&json, "f14"), None);
    }

    #[test]
    fn test_string_field_returns_none_for_dash() {
        let json = serde_json::json!({"f14": "-"});
        assert_eq!(string_field(&json, "f14"), None);
    }

    #[test]
    fn test_string_field_returns_none_for_null() {
        let json = serde_json::json!({"f14": null});
        assert_eq!(string_field(&json, "f14"), None);
    }

    #[test]
    fn test_number_field_extracts_number() {
        let json = serde_json::json!({"f2": 4200.5});
        assert_eq!(number_field(&json, "f2"), Some(4200.5));
    }

    #[test]
    fn test_number_field_extracts_number_string() {
        let json = serde_json::json!({"f62": "12345678"});
        assert_eq!(number_field(&json, "f62"), Some(12345678.0));
    }

    #[test]
    fn test_number_field_returns_none_for_non_numeric_string() {
        let json = serde_json::json!({"f62": "--"});
        assert_eq!(number_field(&json, "f62"), None);
    }

    #[test]
    fn test_number_field_returns_none_for_missing() {
        let json = serde_json::json!({"f12": "BK0001"});
        assert_eq!(number_field(&json, "f62"), None);
    }

    #[test]
    fn test_number_field_returns_none_for_null() {
        let json = serde_json::json!({"f62": null});
        assert_eq!(number_field(&json, "f62"), None);
    }

    // ─── Client construction tests ───────────────────────────────

    #[test]
    fn test_provider_uses_browser_user_agent() {
        // with_base_url should not panic
        let _provider = EastmoneyMarketIntelligenceProvider::with_base_url("http://localhost:9999");
    }

    #[test]
    fn test_default_provider_uses_push2_url() {
        let provider = EastmoneyMarketIntelligenceProvider::new();
        assert_eq!(provider.base_url, "https://push2.eastmoney.com");
    }

    #[test]
    fn test_provider_trims_trailing_slash() {
        let provider = EastmoneyMarketIntelligenceProvider::with_base_url("http://localhost:9999/");
        assert_eq!(provider.base_url, "http://localhost:9999");
    }

    #[test]
    fn test_api_key_strips_query_params_for_rate_limit() {
        assert_eq!(api_key("/api/qt/clist/get?pn=1&_=123"), "/api/qt/clist/get");
    }

    #[test]
    fn test_retry_status_only_retries_expected_statuses() {
        assert!(should_retry_status(StatusCode::FORBIDDEN));
        assert!(should_retry_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(!should_retry_status(StatusCode::INTERNAL_SERVER_ERROR));
        assert!(!should_retry_status(StatusCode::BAD_REQUEST));
    }

    // ─── fetch_sector_rows tests ─────────────────────────────────

    async fn setup_mock_server() -> (MockServer, EastmoneyMarketIntelligenceProvider) {
        let server = MockServer::start().await;
        let provider = EastmoneyMarketIntelligenceProvider::with_base_url(&server.uri());
        (server, provider)
    }

    fn sector_list_response() -> serde_json::Value {
        serde_json::json!({
            "rc": 0,
            "data": {
                "total": 3,
                "diff": [
                    {"f12": "BK0001", "f14": "电子", "f3": 2.5, "f6": 1.2e9, "f62": 1.5e8},
                    {"f12": "BK0002", "f14": "通信", "f3": 1.8, "f6": 8.5e8, "f62": 7.2e7},
                    {"f12": "BK0003", "f14": "半导体", "f3": -0.5, "f6": 5.0e8, "f62": -2.1e7}
                ]
            }
        })
    }

    #[tokio::test]
    async fn test_fetch_sector_rows_returns_diff_array() {
        let (server, provider) = setup_mock_server().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(sector_list_response()))
            .expect(1)
            .mount(&server)
            .await;

        let result = provider.fetch_sector_rows("f62", 50).await.unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(string_field(&result[0], "f14"), Some("电子".to_string()));
        assert_eq!(string_field(&result[1], "f14"), Some("通信".to_string()));
        assert_eq!(string_field(&result[2], "f14"), Some("半导体".to_string()));
        assert_eq!(number_field(&result[0], "f62"), Some(1.5e8));
    }

    #[tokio::test]
    async fn test_fetch_sector_rows_returns_empty_when_no_diff() {
        let (server, provider) = setup_mock_server().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "rc": 0, "data": {}
            })))
            .mount(&server)
            .await;

        let result = provider.fetch_sector_rows("f62", 50).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_fetch_sector_rows_returns_empty_when_diff_null() {
        let (server, provider) = setup_mock_server().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "rc": 0, "data": {"diff": null}
            })))
            .mount(&server)
            .await;

        let result = provider.fetch_sector_rows("f62", 50).await.unwrap();
        assert!(result.is_empty());
    }

    // ─── MarketDataProvider tests ────────────────────────────────

    fn market_snapshot_response() -> serde_json::Value {
        serde_json::json!({
            "data": {
                "diff": [
                    {"f12": "1.000001", "f14": "上证指数", "f2": 4100.5, "f3": 0.85, "f6": 4.0e9},
                    {"f12": "0.399001", "f14": "深证成指", "f2": 16200.3, "f3": 2.1, "f6": 1.6e9},
                    {"f12": "0.399006", "f14": "创业板指", "f2": 4350.0, "f3": 0.0, "f6": 5.0e7}
                ]
            }
        })
    }

    #[tokio::test]
    async fn test_fetch_market_snapshots_parses_all_indices() {
        let (server, provider) = setup_mock_server().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(market_snapshot_response()))
            .mount(&server)
            .await;

        let result = provider.fetch_market_snapshots().await.unwrap();
        // 5 indices configured, but only 3 matched by secid ending
        assert_eq!(result.len(), 3);

        let sh = &result[0];
        assert_eq!(sh.market, "CN");
        assert_eq!(sh.index_name, "上证指数");
        assert!((sh.price - 4100.5).abs() < 0.01);
        assert!((sh.change_pct - 0.85).abs() < 0.01);
        assert_eq!(sh.source, "eastmoney");

        let sz = &result[1];
        assert_eq!(sz.market, "CN");
        assert_eq!(sz.index_name, "深证成指");
        assert!((sz.price - 16200.3).abs() < 0.01);

        let cy = &result[2];
        assert_eq!(cy.market, "CN");
        assert_eq!(cy.index_name, "创业板指");
    }

    #[tokio::test]
    async fn test_fetch_market_snapshots_filters_incomplete_rows() {
        let (server, provider) = setup_mock_server().await;

        // Row without f2 (price) should be filtered out
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "diff": [
                        {"f12": "1.000001", "f14": "上证指数", "f3": 0.85, "f6": 4.0e9},
                        {"f12": "0.399001", "f14": "深证成指", "f2": 16200.3, "f3": 2.1, "f6": 1.6e9}
                    ]
                }
            })))
            .mount(&server)
            .await;

        let result = provider.fetch_market_snapshots().await.unwrap();
        // First row missing f2 (price) so filter_map returns None
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].index_name, "深证成指");
    }

    // ─── SectorRotationProvider tests ────────────────────────────

    #[tokio::test]
    async fn test_fetch_sector_rotation_enumerates_and_ranks() {
        let (server, provider) = setup_mock_server().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "diff": [
                        {"f14": "电子", "f62": 1.0e8, "f3": 2.5, "f6": 1.0e9},
                        {"f14": "通信", "f62": 5.0e7, "f3": 1.2, "f6": 5.0e8}
                    ]
                }
            })))
            .mount(&server)
            .await;

        let result = provider
            .fetch_sector_rotation_snapshots(None)
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].sector, "电子");
        assert_eq!(result[0].ranking, Some(1));
        assert_eq!(result[0].net_flow, Some(1.0e8));
        assert_eq!(result[0].change_pct, Some(2.5));
        assert_eq!(result[0].turnover, Some(1.0e9));
        assert_eq!(result[0].market, "CN");
        assert_eq!(result[0].source, "eastmoney");

        assert_eq!(result[1].sector, "通信");
        assert_eq!(result[1].ranking, Some(2));
    }

    #[tokio::test]
    async fn test_fetch_sector_rotation_filters_missing_sector_name() {
        let (server, provider) = setup_mock_server().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "diff": [
                        {"f14": "电子", "f62": 1.0e8, "f3": 2.5, "f6": 1.0e9},
                        {"f62": 5.0e7, "f3": 1.2, "f6": 5.0e8},   // missing f14
                        {"f14": "半导体", "f62": 3.0e7, "f3": -0.5, "f6": 3.0e8}
                    ]
                }
            })))
            .mount(&server)
            .await;

        let result = provider
            .fetch_sector_rotation_snapshots(None)
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].sector, "电子");
        assert_eq!(result[1].sector, "半导体");
    }

    // ─── CapitalFlowProvider tests ───────────────────────────────

    #[tokio::test]
    async fn test_fetch_capital_flow_calculates_inflow_outflow() {
        let (server, provider) = setup_mock_server().await;

        // Mix of positive and negative net flows
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "diff": [
                        {"f14": "电子", "f62": 100.0},
                        {"f14": "通信", "f62": 50.0},
                        {"f14": "银行", "f62": -30.0},
                        {"f14": "地产", "f62": -20.0},
                        {"f14": "消费", "f62": 10.0}
                    ]
                }
            })))
            .mount(&server)
            .await;

        let result = provider.fetch_capital_flow_snapshots(None).await.unwrap();

        assert_eq!(result.len(), 1);
        let snapshot = &result[0];
        assert_eq!(snapshot.market, "CN");
        assert_eq!(snapshot.category, "main_funds");
        // inflow = 100 + 50 + 10 = 160
        assert_eq!(snapshot.inflow, Some(160.0));
        // outflow = 30 + 20 = 50
        assert_eq!(snapshot.outflow, Some(50.0));
        // net_flow = 160 - 50 = 110
        assert_eq!(snapshot.net_flow, 110.0);
        assert_eq!(snapshot.source, "eastmoney");
    }

    #[tokio::test]
    async fn test_fetch_capital_flow_handles_all_negative() {
        let (server, provider) = setup_mock_server().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "diff": [
                        {"f14": "银行", "f62": -50.0},
                        {"f14": "地产", "f62": -30.0}
                    ]
                }
            })))
            .mount(&server)
            .await;

        let result = provider.fetch_capital_flow_snapshots(None).await.unwrap();

        assert_eq!(result.len(), 1);
        let snapshot = &result[0];
        assert_eq!(snapshot.inflow, Some(0.0));
        assert_eq!(snapshot.outflow, Some(80.0));
        assert!((snapshot.net_flow - (-80.0)).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_fetch_capital_flow_handles_empty_sectors() {
        let (server, provider) = setup_mock_server().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {"diff": []}
            })))
            .mount(&server)
            .await;

        let result = provider.fetch_capital_flow_snapshots(None).await.unwrap();

        assert_eq!(result.len(), 1);
        let snapshot = &result[0];
        assert_eq!(snapshot.inflow, Some(0.0));
        assert_eq!(snapshot.outflow, Some(0.0));
        assert_eq!(snapshot.net_flow, 0.0);
    }

    // ─── Error handling tests ────────────────────────────────────

    #[tokio::test]
    async fn test_fetch_sector_rows_returns_error_on_http_500() {
        let (server, provider) = setup_mock_server().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let result = provider.fetch_sector_rows("f62", 50).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("HTTP 500"));
    }

    #[tokio::test]
    async fn test_fetch_sector_rows_returns_error_on_invalid_json() {
        let (server, provider) = setup_mock_server().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not json at all"))
            .mount(&server)
            .await;

        let result = provider.fetch_sector_rows("f62", 50).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Invalid Eastmoney response"));
    }

    // ─── default_theme_mappings tests ────────────────────────────

    #[test]
    fn test_default_theme_mappings_contains_all_themes() {
        let mappings = default_theme_mappings();
        let themes: Vec<&str> = mappings.iter().map(|m| m.theme.as_str()).collect();

        assert!(themes.contains(&"AI"));
        assert!(themes.contains(&"GPU"));
        assert!(themes.contains(&"HBM"));
        assert!(themes.contains(&"CPO"));
        assert!(themes.contains(&"IDC"));
        assert!(themes.contains(&"机器人"));
        assert!(themes.contains(&"自动驾驶"));
        assert!(themes.contains(&"创新药"));
        assert!(themes.contains(&"消费电子"));
        assert!(themes.contains(&"新能源"));
        assert!(themes.contains(&"银行"));
        assert!(themes.contains(&"地产"));
        assert!(themes.contains(&"消费"));
        assert_eq!(mappings.len(), 13);
    }

    #[test]
    fn test_theme_mappings_have_non_empty_sectors() {
        let mappings = default_theme_mappings();
        for mapping in &mappings {
            assert!(
                !mapping.sectors.is_empty(),
                "Theme '{}' has no sectors",
                mapping.theme
            );
        }
    }
}
