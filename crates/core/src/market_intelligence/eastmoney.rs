use std::{
    collections::HashMap,
    sync::OnceLock,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use chrono::{NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Asia::Shanghai;
use reqwest::{header, StatusCode};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::errors::{Error, Result};

use super::{
    capital_flow::{CapitalFlowProvider, CapitalFlowSnapshot},
    intraday::{IntradayMarketIntelligenceProvider, MarketIntelligenceIntradaySnapshot},
    macro_capital::{models::indicator, MacroCapitalProvider, MacroCapitalSnapshot},
    market_overview::{MarketDataProvider, MarketSnapshot},
    sector_rotation::{SectorRotationProvider, SectorRotationSnapshot},
    theme_rotation::{ThemeMapping, ThemeMappingProvider},
};

const SOURCE: &str = "eastmoney";
const PUSH2_BASE_URL: &str = "https://push2.eastmoney.com";
const PUSH2HIS_BASE_URL: &str = "https://push2his.eastmoney.com";
const DATACENTER_BASE_URL: &str = "https://datacenter-web.eastmoney.com";
const DATACENTER_SOURCE: &str = "eastmoney_datacenter";
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
        self.get_json_from_base(&self.base_url, path).await
    }

    async fn get_his_json(&self, path: &str) -> Result<Value> {
        let base_url = if self.uses_default_base_url() {
            PUSH2HIS_BASE_URL
        } else {
            &self.base_url
        };
        self.get_json_from_base(base_url, path).await
    }

    async fn get_json_from_base(&self, base_url: &str, path: &str) -> Result<Value> {
        if self.uses_default_base_url() {
            self.ensure_session().await?;
        }
        rate_limit(api_key(path)).await;

        let url = format!("{}{}", base_url, path);
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

    async fn fetch_sector_rows(&self, fs: &str, fid: &str, limit: usize) -> Result<Vec<Value>> {
        let path = format!(
            "/api/qt/clist/get?pn=1&pz={limit}&po=1&np=1&fltt=2&invt=2&fid={fid}&fs={fs}&fields=f12,f13,f14,f3,f6,f62&ut={EASTMONEY_UT}&_={}",
            Utc::now().timestamp_millis()
        );
        let value = self.get_json(&path).await?;
        Ok(value
            .pointer("/data/diff")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default())
    }

    /// Fetch both industry (m:90+t:2) and concept (m:90+t:3) board rows,
    /// deduplicating by secid so each board appears only once.
    async fn fetch_all_sector_rows(
        &self,
        fid: &str,
        industry_limit: usize,
        concept_limit: usize,
    ) -> Result<Vec<Value>> {
        let (industry, concept) = tokio::join!(
            self.fetch_sector_rows("m:90+t:2", fid, industry_limit),
            self.fetch_sector_rows("m:90+t:3", fid, concept_limit),
        );
        let industry = industry.unwrap_or_default();
        let concept = concept.unwrap_or_default();

        let mut unique = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for row in industry.into_iter().chain(concept) {
            if let Some(code) = string_field(&row, "f12") {
                let secid = eastmoney_secid(&row, &code);
                if seen.insert(secid) {
                    unique.push(row);
                }
            }
        }
        Ok(unique)
    }

    async fn fetch_sector_minute_klines(
        &self,
        secid: &str,
        sector: &str,
        market: &str,
        ranking: Option<i32>,
        date: NaiveDate,
    ) -> Result<Vec<MarketIntelligenceIntradaySnapshot>> {
        let day = date.format("%Y%m%d").to_string();
        let path = format!(
            "/api/qt/stock/kline/get?secid={secid}&klt=1&fqt=1&beg={day}&end={day}&fields1=f1,f2,f3,f4,f5,f6&fields2=f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61&rtntype=6&ut={EASTMONEY_UT}&_={}",
            Utc::now().timestamp_millis()
        );
        let value = self.get_his_json(&path).await?;
        let rows = value
            .pointer("/data/klines")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        Ok(rows
            .iter()
            .filter_map(Value::as_str)
            .filter_map(|row| parse_minute_kline(row, sector, market, ranking))
            .collect())
    }

    fn uses_default_base_url(&self) -> bool {
        self.base_url == PUSH2_BASE_URL
    }

    /// Fetch JSON from the Eastmoney datacenter host, reusing the same browser
    /// client, rate limiter and retry logic as the push2 endpoints. When a
    /// custom base URL is configured (tests), requests route there instead so
    /// the datacenter flows stay mockable.
    async fn get_datacenter_json(&self, path: &str) -> Result<Value> {
        let base_url = if self.uses_default_base_url() {
            DATACENTER_BASE_URL
        } else {
            &self.base_url
        };
        self.get_json_from_base(base_url, path).await
    }

    /// Fetch the newest rows from a datacenter report, sorted descending by the
    /// given column. Returns an empty vector when the report has no data.
    async fn fetch_datacenter_rows(
        &self,
        report_name: &str,
        sort_column: &str,
        limit: usize,
    ) -> Result<Vec<Value>> {
        let path = format!(
            "/api/data/v1/get?reportName={report_name}&columns=ALL&sortColumns={sort_column}&sortTypes=-1&source=WEB&client=WEB&pageNumber=1&pageSize={limit}&_={}",
            Utc::now().timestamp_millis()
        );
        let value = self.get_datacenter_json(&path).await?;
        Ok(value
            .pointer("/result/data")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default())
    }

    /// Northbound / southbound Stock Connect net flow for the latest trade date.
    /// Best effort: returns whatever directions the datacenter report exposes.
    async fn fetch_connect_flow_snapshots(&self) -> Result<Vec<MacroCapitalSnapshot>> {
        let rows = self
            .fetch_datacenter_rows("RPT_MUTUAL_DEAL_HISTORY", "TRADE_DATE", 16)
            .await?;
        Ok(parse_connect_flow_rows(&rows))
    }

    /// Market-wide margin financing + securities lending balance (两融余额)
    /// for the latest available date. Best effort.
    async fn fetch_margin_balance_snapshot(&self) -> Result<Option<MacroCapitalSnapshot>> {
        let rows = self
            .fetch_datacenter_rows("RPTA_RZRQ_LSHJ", "dim_date", 2)
            .await?;
        Ok(parse_margin_balance_rows(&rows))
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

#[async_trait]
impl IntradayMarketIntelligenceProvider for EastmoneyMarketIntelligenceProvider {
    async fn fetch_intraday_snapshots(
        &self,
        date: Option<NaiveDate>,
    ) -> Result<Vec<MarketIntelligenceIntradaySnapshot>> {
        let snapshot_date = date.unwrap_or_else(|| Utc::now().date_naive());
        let rows = self.fetch_all_sector_rows("f62", 100, 100).await?;
        let now = Utc::now();
        let mut snapshots = rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| {
                Some(MarketIntelligenceIntradaySnapshot {
                    market: "CN".to_string(),
                    kind: "sector_rotation".to_string(),
                    name: string_field(row, "f14")?,
                    timestamp: now,
                    open: None,
                    close: None,
                    high: None,
                    low: None,
                    volume: None,
                    amount: number_field(row, "f6"),
                    net_flow: number_field(row, "f62"),
                    change_pct: number_field(row, "f3"),
                    turnover: None,
                    ranking: Some((index + 1) as i32),
                    source: "eastmoney_realtime".to_string(),
                })
            })
            .collect::<Vec<_>>();

        for (index, row) in rows.iter().take(12).enumerate() {
            let Some(code) = string_field(row, "f12") else {
                continue;
            };
            let Some(sector) = string_field(row, "f14") else {
                continue;
            };
            let secid = eastmoney_secid(row, &code);
            match self
                .fetch_sector_minute_klines(
                    &secid,
                    &sector,
                    "CN",
                    Some((index + 1) as i32),
                    snapshot_date,
                )
                .await
            {
                Ok(mut rows) => snapshots.append(&mut rows),
                Err(error) => {
                    log::debug!("Failed to fetch Eastmoney minute kline {secid}: {error}")
                }
            }
        }

        Ok(snapshots)
    }
}

fn eastmoney_client() -> &'static reqwest::Client {
    EASTMONEY_CLIENT.get_or_init(|| {
        // Keep a single browser-like transport for the module: cookies,
        // decompression, and keep-alive are handled by reqwest rather than
        // copied into per-request headers.
        reqwest::Client::builder()
            .user_agent(BROWSER_USER_AGENT)
            // Eastmoney's WAF intermittently rejects HTTP/2 handshakes, which
            // surfaces as "error sending request for url". Pin the transport to
            // HTTP/1.1 so requests match the browser/curl fingerprint reliably.
            .http1_only()
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
        let rows = self.fetch_all_sector_rows("f62", 100, 100).await?;
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
        let rows = self.fetch_all_sector_rows("f62", 200, 300).await?;
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

#[async_trait]
impl MacroCapitalProvider for EastmoneyMarketIntelligenceProvider {
    async fn fetch_macro_capital_snapshots(
        &self,
        _date: Option<NaiveDate>,
    ) -> Result<Vec<MacroCapitalSnapshot>> {
        let mut snapshots = Vec::new();

        // Northbound / southbound connect flows (best effort). A broken upstream
        // endpoint degrades to "no data" instead of failing the whole refresh.
        match self.fetch_connect_flow_snapshots().await {
            Ok(mut rows) => snapshots.append(&mut rows),
            Err(error) => log::warn!("Failed to fetch Eastmoney connect flows: {error}"),
        }

        // Market-wide margin balance, best effort.
        match self.fetch_margin_balance_snapshot().await {
            Ok(Some(row)) => snapshots.push(row),
            Ok(None) => {}
            Err(error) => log::warn!("Failed to fetch Eastmoney margin balance: {error}"),
        }

        // TODO(macro-capital): ETF net subscription/redemption and ETF share
        // changes have no stable free datacenter report reused here yet. DXY and
        // VIX are sourced from the shared QuoteService in the service layer, not
        // from Eastmoney, so they are intentionally not fetched here.
        Ok(snapshots)
    }
}

/// Look up the first present, non-empty string field from candidate keys
/// (datacenter reports rename columns over time).
fn first_string_field(row: &Value, fields: &[&str]) -> Option<String> {
    fields.iter().find_map(|field| string_field(row, field))
}

/// Look up the first present numeric field from candidate keys.
fn first_number_field(row: &Value, fields: &[&str]) -> Option<f64> {
    fields.iter().find_map(|field| number_field(row, field))
}

fn parse_datacenter_date(value: &str) -> Option<NaiveDate> {
    let trimmed = value.split_whitespace().next().unwrap_or(value);
    NaiveDate::parse_from_str(trimmed, "%Y-%m-%d").ok()
}

/// Classify a Stock Connect `MUTUAL_TYPE` code into a flow direction.
/// 001/003 -> northbound; 002/004 -> southbound.
fn connect_direction(mutual_type: &str) -> Option<&'static str> {
    match mutual_type {
        "001" | "003" => Some(indicator::NORTHBOUND),
        "002" | "004" => Some(indicator::SOUTHBOUND),
        _ => None,
    }
}

/// Aggregate the latest trade date's northbound and southbound net deal amounts
/// from an `RPT_MUTUAL_DEAL_HISTORY` response.
fn parse_connect_flow_rows(rows: &[Value]) -> Vec<MacroCapitalSnapshot> {
    let dated = rows
        .iter()
        .filter_map(|row| {
            let date = parse_datacenter_date(&first_string_field(row, &["TRADE_DATE"])?)?;
            Some((date, row))
        })
        .collect::<Vec<_>>();
    let Some(latest_date) = dated.iter().map(|(date, _)| *date).max() else {
        return Vec::new();
    };

    let mut north: Option<f64> = None;
    let mut south: Option<f64> = None;
    for (_, row) in dated.iter().filter(|(date, _)| *date == latest_date) {
        let Some(net) = first_number_field(row, &["NET_DEAL_AMT", "NET_BUY_AMT", "FUND_INFLOW"])
        else {
            continue;
        };
        match first_string_field(row, &["MUTUAL_TYPE", "BOARD_TYPE"]).as_deref() {
            Some(mutual_type) => match connect_direction(mutual_type) {
                Some(indicator::NORTHBOUND) => *north.get_or_insert(0.0) += net,
                Some(indicator::SOUTHBOUND) => *south.get_or_insert(0.0) += net,
                _ => {}
            },
            // Reports without a type column expose a single (northbound) series.
            None => *north.get_or_insert(0.0) += net,
        }
    }

    let mut snapshots = Vec::new();
    if let Some(value) = north {
        snapshots.push(MacroCapitalSnapshot {
            indicator: indicator::NORTHBOUND.to_string(),
            market: "CN".to_string(),
            date: latest_date,
            value,
            change: None,
            unit: Some("100M_CNY".to_string()),
            source: DATACENTER_SOURCE.to_string(),
        });
    }
    if let Some(value) = south {
        snapshots.push(MacroCapitalSnapshot {
            indicator: indicator::SOUTHBOUND.to_string(),
            market: "HK".to_string(),
            date: latest_date,
            value,
            change: None,
            unit: Some("100M_CNY".to_string()),
            source: DATACENTER_SOURCE.to_string(),
        });
    }
    snapshots
}

/// Parse the latest market-wide margin balance into 亿元, computing the
/// day-over-day change when a previous row is available.
fn parse_margin_balance_rows(rows: &[Value]) -> Option<MacroCapitalSnapshot> {
    let mut dated = rows
        .iter()
        .filter_map(|row| {
            let date = parse_datacenter_date(&first_string_field(
                row,
                &["DIM_DATE", "TRADE_DATE", "STATISTICS_DATE"],
            )?)?;
            let raw = first_number_field(row, &["RZRQYE", "RZRQ_BALANCE", "FIN_BALANCE"])?;
            Some((date, raw))
        })
        .collect::<Vec<_>>();
    dated.sort_by_key(|(date, _)| *date);

    let (date, raw) = dated.last().copied()?;
    let previous = dated.iter().rev().nth(1).map(|(_, raw)| *raw);
    // Datacenter reports the balance in 元; present it in 亿元 for the UI.
    let value = raw / 1e8;
    let change = previous.map(|previous| (raw - previous) / 1e8);
    Some(MacroCapitalSnapshot {
        indicator: indicator::MARGIN_BALANCE.to_string(),
        market: "CN".to_string(),
        date,
        value,
        change,
        unit: Some("100M_CNY_balance".to_string()),
        source: DATACENTER_SOURCE.to_string(),
    })
}

pub fn default_theme_mappings() -> Vec<ThemeMapping> {
    vec![
        mapping(
            "AI",
            &[
                "计算机",
                "软件开发",
                "通信设备",
                "光学光电子",
                "半导体",
                "人工智能",
                "AI芯片",
                "ChatGPT概念",
                "AIGC概念",
                "大模型",
                "算力概念",
            ],
        ),
        mapping(
            "GPU",
            &[
                "半导体",
                "数字芯片设计",
                "模拟芯片设计",
                "集成电路封测",
                "GPU",
                "AI芯片",
                "先进封装",
            ],
        ),
        mapping("HBM", &["半导体", "存储芯片", "数字芯片设计", "HBM概念"]),
        mapping(
            "CPO",
            &[
                "通信设备",
                "光模块",
                "光学光电子",
                "通信网络设备及器件",
                "CPO概念",
                "共封装光学",
            ],
        ),
        mapping(
            "IDC",
            &["计算机", "通信服务", "互联网服务", "数据中心", "东数西算"],
        ),
        mapping(
            "机器人",
            &[
                "机器人",
                "机器人概念",
                "人形机器人",
                "自动化设备",
                "通用设备",
            ],
        ),
        mapping(
            "自动驾驶",
            &["汽车零部件", "汽车服务", "软件开发", "无人驾驶", "智能驾驶"],
        ),
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

fn eastmoney_secid(row: &Value, code: &str) -> String {
    let market = row
        .get("f13")
        .and_then(Value::as_i64)
        .map(|value| value.to_string())
        .or_else(|| string_field(row, "f13"))
        .unwrap_or_else(|| {
            if code.starts_with('6') {
                "1".to_string()
            } else if code.starts_with("BK") {
                "90".to_string()
            } else {
                "0".to_string()
            }
        });
    format!("{market}.{code}")
}

fn parse_minute_kline(
    row: &str,
    sector: &str,
    market: &str,
    ranking: Option<i32>,
) -> Option<MarketIntelligenceIntradaySnapshot> {
    let parts = row.split(',').collect::<Vec<_>>();
    let timestamp = parse_eastmoney_minute_timestamp(parts.first().copied()?)?;
    Some(MarketIntelligenceIntradaySnapshot {
        market: market.to_string(),
        kind: "sector_rotation".to_string(),
        name: sector.to_string(),
        timestamp,
        open: parse_kline_number(parts.get(1).copied()),
        close: parse_kline_number(parts.get(2).copied()),
        high: parse_kline_number(parts.get(3).copied()),
        low: parse_kline_number(parts.get(4).copied()),
        volume: parse_kline_number(parts.get(5).copied()),
        amount: parse_kline_number(parts.get(6).copied()),
        net_flow: None,
        change_pct: parse_kline_number(parts.get(8).copied()),
        turnover: parse_kline_number(parts.get(10).copied()),
        ranking,
        source: "eastmoney_kline_1m".to_string(),
    })
}

fn parse_eastmoney_minute_timestamp(value: &str) -> Option<chrono::DateTime<Utc>> {
    let naive = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M")
        .ok()
        .or_else(|| {
            NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .ok()
                .and_then(|date| date.and_hms_opt(0, 0, 0))
        })?;
    Shanghai
        .from_local_datetime(&naive)
        .single()
        .map(|value| value.with_timezone(&Utc))
}

fn parse_kline_number(value: Option<&str>) -> Option<f64> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "-")
        .and_then(|value| value.parse::<f64>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
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

    #[test]
    fn test_eastmoney_secid_uses_f13_when_available() {
        let row = serde_json::json!({"f12": "BK1036", "f13": 90});
        assert_eq!(eastmoney_secid(&row, "BK1036"), "90.BK1036");
    }

    #[test]
    fn test_parse_minute_kline_builds_intraday_snapshot() {
        let snapshot = parse_minute_kline(
            "2026-07-03 09:31,10.1,10.2,10.3,10.0,1234,567890,1.2,0.8,0.08,2.1",
            "半导体",
            "CN",
            Some(1),
        )
        .unwrap();

        assert_eq!(snapshot.kind, "sector_rotation");
        assert_eq!(snapshot.name, "半导体");
        assert_eq!(snapshot.close, Some(10.2));
        assert_eq!(snapshot.change_pct, Some(0.8));
        assert_eq!(snapshot.turnover, Some(2.1));
        assert_eq!(snapshot.source, "eastmoney_kline_1m");
    }

    // ─── fetch_sector_rows tests ─────────────────────────────────

    async fn setup_mock_server() -> Option<(MockServer, EastmoneyMarketIntelligenceProvider)> {
        let listener = match TcpListener::bind("127.0.0.1:0") {
            Ok(listener) => listener,
            Err(error) => {
                eprintln!("Skipping Eastmoney HTTP mock test: cannot bind local port ({error})");
                return None;
            }
        };
        let server = MockServer::builder().listener(listener).start().await;
        let provider = EastmoneyMarketIntelligenceProvider::with_base_url(&server.uri());
        Some((server, provider))
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
        let Some((server, provider)) = setup_mock_server().await else {
            return;
        };

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(sector_list_response()))
            .expect(1)
            .mount(&server)
            .await;

        let result = provider
            .fetch_sector_rows("m:90+t:2", "f62", 50)
            .await
            .unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(string_field(&result[0], "f14"), Some("电子".to_string()));
        assert_eq!(string_field(&result[1], "f14"), Some("通信".to_string()));
        assert_eq!(string_field(&result[2], "f14"), Some("半导体".to_string()));
        assert_eq!(number_field(&result[0], "f62"), Some(1.5e8));
    }

    #[tokio::test]
    async fn test_fetch_sector_rows_returns_empty_when_no_diff() {
        let Some((server, provider)) = setup_mock_server().await else {
            return;
        };

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "rc": 0, "data": {}
            })))
            .mount(&server)
            .await;

        let result = provider
            .fetch_sector_rows("m:90+t:2", "f62", 50)
            .await
            .unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_fetch_sector_rows_returns_empty_when_diff_null() {
        let Some((server, provider)) = setup_mock_server().await else {
            return;
        };

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "rc": 0, "data": {"diff": null}
            })))
            .mount(&server)
            .await;

        let result = provider
            .fetch_sector_rows("m:90+t:2", "f62", 50)
            .await
            .unwrap();
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
        let Some((server, provider)) = setup_mock_server().await else {
            return;
        };

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
        let Some((server, provider)) = setup_mock_server().await else {
            return;
        };

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
        let Some((server, provider)) = setup_mock_server().await else {
            return;
        };

        let body = serde_json::json!({
            "data": {
                "diff": [
                    {"f12": "BK0001", "f13": 90, "f14": "电子", "f62": 1.0e8, "f3": 2.5, "f6": 1.0e9},
                    {"f12": "BK0002", "f13": 90, "f14": "通信", "f62": 5.0e7, "f3": 1.2, "f6": 5.0e8}
                ]
            }
        });
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
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
        let Some((server, provider)) = setup_mock_server().await else {
            return;
        };

        let body = serde_json::json!({
            "data": {
                "diff": [
                    {"f12": "BK0001", "f13": 90, "f14": "电子", "f62": 1.0e8, "f3": 2.5, "f6": 1.0e9},
                    {"f12": "BK0002", "f13": 90, "f62": 5.0e7, "f3": 1.2, "f6": 5.0e8},
                    {"f12": "BK0003", "f13": 90, "f14": "半导体", "f62": 3.0e7, "f3": -0.5, "f6": 3.0e8}
                ]
            }
        });
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
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
        let Some((server, provider)) = setup_mock_server().await else {
            return;
        };

        let body = serde_json::json!({
            "data": {
                "diff": [
                    {"f12": "BK0001", "f13": 90, "f14": "电子", "f62": 100.0},
                    {"f12": "BK0002", "f13": 90, "f14": "通信", "f62": 50.0},
                    {"f12": "BK0003", "f13": 90, "f14": "银行", "f62": -30.0},
                    {"f12": "BK0004", "f13": 90, "f14": "地产", "f62": -20.0},
                    {"f12": "BK0005", "f13": 90, "f14": "消费", "f62": 10.0}
                ]
            }
        });
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
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
        let Some((server, provider)) = setup_mock_server().await else {
            return;
        };

        let body = serde_json::json!({
            "data": {
                "diff": [
                    {"f12": "BK0003", "f13": 90, "f14": "银行", "f62": -50.0},
                    {"f12": "BK0004", "f13": 90, "f14": "地产", "f62": -30.0}
                ]
            }
        });
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
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
        let Some((server, provider)) = setup_mock_server().await else {
            return;
        };

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
        let Some((server, provider)) = setup_mock_server().await else {
            return;
        };

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let result = provider.fetch_sector_rows("m:90+t:2", "f62", 50).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("HTTP 500"));
    }

    #[tokio::test]
    async fn test_fetch_sector_rows_returns_error_on_invalid_json() {
        let Some((server, provider)) = setup_mock_server().await else {
            return;
        };

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not json at all"))
            .mount(&server)
            .await;

        let result = provider.fetch_sector_rows("m:90+t:2", "f62", 50).await;
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

    // ─── MacroCapitalProvider parsing tests ──────────────────────

    #[test]
    fn test_parse_connect_flow_rows_aggregates_latest_date_by_direction() {
        let rows = vec![
            serde_json::json!({"TRADE_DATE": "2026-07-08 00:00:00", "MUTUAL_TYPE": "001", "NET_DEAL_AMT": 30.0}),
            serde_json::json!({"TRADE_DATE": "2026-07-08 00:00:00", "MUTUAL_TYPE": "003", "NET_DEAL_AMT": 20.0}),
            serde_json::json!({"TRADE_DATE": "2026-07-08 00:00:00", "MUTUAL_TYPE": "002", "NET_DEAL_AMT": -5.0}),
            serde_json::json!({"TRADE_DATE": "2026-07-07 00:00:00", "MUTUAL_TYPE": "001", "NET_DEAL_AMT": 999.0}),
        ];
        let result = parse_connect_flow_rows(&rows);
        assert_eq!(result.len(), 2);
        let north = result
            .iter()
            .find(|s| s.indicator == indicator::NORTHBOUND)
            .unwrap();
        assert_eq!(north.value, 50.0);
        assert_eq!(north.market, "CN");
        assert_eq!(north.date, NaiveDate::from_ymd_opt(2026, 7, 8).unwrap());
        let south = result
            .iter()
            .find(|s| s.indicator == indicator::SOUTHBOUND)
            .unwrap();
        assert_eq!(south.value, -5.0);
        assert_eq!(south.market, "HK");
    }

    #[test]
    fn test_parse_connect_flow_rows_handles_single_series_without_type() {
        let rows = vec![serde_json::json!({"TRADE_DATE": "2026-07-08", "NET_DEAL_AMT": 42.0})];
        let result = parse_connect_flow_rows(&rows);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].indicator, indicator::NORTHBOUND);
        assert_eq!(result[0].value, 42.0);
    }

    #[test]
    fn test_parse_connect_flow_rows_empty_when_no_rows() {
        assert!(parse_connect_flow_rows(&[]).is_empty());
    }

    #[test]
    fn test_parse_margin_balance_rows_converts_to_100m_and_computes_change() {
        let rows = vec![
            serde_json::json!({"DIM_DATE": "2026-07-08", "RZRQYE": 1.85e12}),
            serde_json::json!({"DIM_DATE": "2026-07-07", "RZRQYE": 1.80e12}),
        ];
        let snapshot = parse_margin_balance_rows(&rows).unwrap();
        assert_eq!(snapshot.indicator, indicator::MARGIN_BALANCE);
        assert!((snapshot.value - 18500.0).abs() < 0.01);
        assert!((snapshot.change.unwrap() - 500.0).abs() < 0.01);
        assert_eq!(snapshot.date, NaiveDate::from_ymd_opt(2026, 7, 8).unwrap());
    }

    #[test]
    fn test_parse_margin_balance_rows_none_when_empty() {
        assert!(parse_margin_balance_rows(&[]).is_none());
    }

    #[test]
    fn test_connect_direction_classifies_codes() {
        assert_eq!(connect_direction("001"), Some(indicator::NORTHBOUND));
        assert_eq!(connect_direction("003"), Some(indicator::NORTHBOUND));
        assert_eq!(connect_direction("002"), Some(indicator::SOUTHBOUND));
        assert_eq!(connect_direction("004"), Some(indicator::SOUTHBOUND));
        assert_eq!(connect_direction("999"), None);
    }
}
