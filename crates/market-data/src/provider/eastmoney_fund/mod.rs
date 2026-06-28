//! Eastmoney fund market data provider.
//!
//! Uses Eastmoney public fund pages to fetch mainland China mutual-fund NAV data.

use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use num_traits::FromPrimitive;
use reqwest::header;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::errors::MarketDataError;
use crate::models::{Coverage, InstrumentKind, ProviderInstrument, Quote, QuoteContext};
use crate::provider::{MarketDataProvider, ProviderCapabilities, RateLimit};

const PROVIDER_ID: &str = "EASTMONEY_FUND";
const DEFAULT_BASE_URL: &str = "https://fund.eastmoney.com";
const DEFAULT_F10_BASE_URL: &str = "https://fundf10.eastmoney.com";
const DEFAULT_CURRENCY: &str = "CNY";

#[derive(Clone, Copy, Debug)]
pub enum Period {
    OneYear,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketQuote {
    pub symbol: String,
    pub price: f64,
    pub currency: String,
    pub as_of_date: NaiveDate,
    pub source: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalPrice {
    pub symbol: String,
    pub date: NaiveDate,
    pub close: f64,
    pub currency: String,
    pub source: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundResearchSnapshot {
    pub code: String,
    pub name: String,
    pub short_name: Option<String>,
    pub fund_type: Option<String>,
    pub currency: String,
    pub quote: Option<FundQuote>,
    pub nav_history: Vec<FundNavPoint>,
    pub managers: Vec<FundManager>,
    pub fund_company: Option<String>,
    pub custodian_bank: Option<String>,
    pub holdings: Vec<FundHolding>,
    pub holding_changes: Vec<FundHoldingChange>,
    pub sector_allocations: Vec<SectorAllocation>,
    pub asset_allocations: Vec<AssetAllocation>,
    pub region_allocations: Vec<RegionAllocation>,
    pub performance: Option<FundPerformance>,
    pub risk_metrics: Option<FundRiskMetrics>,
    pub fees: Option<FundFees>,
    pub scale_history: Vec<FundScalePoint>,
    pub holder_structure: Option<HolderStructure>,
    pub announcements: Vec<FundAnnouncement>,
    pub dividend_history: Vec<FundDividend>,
    pub purchase_rules: Option<PurchaseRules>,
    pub ai_features: FundAiFeatures,
    pub source: String,
    pub fetched_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundQuote {
    pub nav: f64,
    pub nav_date: NaiveDate,
    pub daily_return_pct: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundNavPoint {
    pub date: NaiveDate,
    pub nav: f64,
    pub daily_return_pct: Option<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundManager {
    pub name: String,
    pub started_on: Option<NaiveDate>,
    pub tenure_days: Option<u32>,
    pub return_pct: Option<f64>,
    pub current_fund_count: Option<u32>,
    pub current_aum_billion: Option<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundHolding {
    pub stock_code: Option<String>,
    pub stock_name: String,
    pub weight_pct: Option<f64>,
    pub rank: Option<u32>,
    pub report_date: Option<NaiveDate>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HoldingChangeType {
    New,
    Removed,
    Increased,
    Decreased,
    Unchanged,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundHoldingChange {
    pub stock_code: Option<String>,
    pub stock_name: String,
    pub previous_weight_pct: Option<f64>,
    pub current_weight_pct: Option<f64>,
    pub weight_change_pct: Option<f64>,
    pub previous_rank: Option<u32>,
    pub current_rank: Option<u32>,
    pub rank_change: Option<i32>,
    pub change_type: HoldingChangeType,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectorAllocation {
    pub sector: String,
    pub weight_pct: Option<f64>,
    pub previous_weight_pct: Option<f64>,
    pub weight_change_pct: Option<f64>,
    pub report_date: Option<NaiveDate>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetAllocation {
    pub asset_class: String,
    pub weight_pct: Option<f64>,
    pub report_date: Option<NaiveDate>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegionAllocation {
    pub region: String,
    pub weight_pct: Option<f64>,
    pub report_date: Option<NaiveDate>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundPerformance {
    pub return_1w_pct: Option<f64>,
    pub return_1m_pct: Option<f64>,
    pub return_3m_pct: Option<f64>,
    pub return_6m_pct: Option<f64>,
    pub return_1y_pct: Option<f64>,
    pub return_2y_pct: Option<f64>,
    pub return_3y_pct: Option<f64>,
    pub return_ytd_pct: Option<f64>,
    pub return_since_inception_pct: Option<f64>,
    pub peer_rank: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundRiskMetrics {
    pub max_drawdown_pct: Option<f64>,
    pub max_drawdown_1m_pct: Option<f64>,
    pub max_drawdown_3m_pct: Option<f64>,
    pub max_drawdown_6m_pct: Option<f64>,
    pub max_drawdown_1y_pct: Option<f64>,
    pub max_drawdown_3y_pct: Option<f64>,
    pub annualized_volatility_pct: Option<f64>,
    pub downside_volatility_pct: Option<f64>,
    pub sharpe_ratio: Option<f64>,
    pub return_drawdown_ratio: Option<f64>,
    pub calmar_ratio: Option<f64>,
    pub beta: Option<f64>,
    pub alpha: Option<f64>,
    pub tracking_error: Option<f64>,
    pub information_ratio: Option<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundFees {
    pub management_fee_pct: Option<f64>,
    pub custodian_fee_pct: Option<f64>,
    pub sales_service_fee_pct: Option<f64>,
    pub purchase_fee_pct: Option<f64>,
    pub redemption_fee_pct: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundScalePoint {
    pub report_date: NaiveDate,
    pub fund_size_billion: Option<f64>,
    pub share_billion: Option<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HolderStructure {
    pub institutional_pct: Option<f64>,
    pub individual_pct: Option<f64>,
    pub internal_pct: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AnnouncementCategory {
    QuarterlyReport,
    SemiAnnualReport,
    AnnualReport,
    ManagerChange,
    ContractChange,
    Dividend,
    PurchaseSuspension,
    PurchaseResume,
    RiskWarning,
    LiquidationRisk,
    MajorEvent,
    Other,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AnnouncementImportance {
    Low,
    Medium,
    High,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundAnnouncement {
    pub title: String,
    pub date: Option<NaiveDate>,
    pub url: Option<String>,
    pub category: Option<AnnouncementCategory>,
    pub importance: AnnouncementImportance,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundDividend {
    pub date: Option<NaiveDate>,
    pub dividend_per_share: Option<f64>,
    pub registration_date: Option<NaiveDate>,
    pub ex_dividend_date: Option<NaiveDate>,
    pub payment_date: Option<NaiveDate>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseRules {
    pub purchase_status: Option<String>,
    pub redemption_status: Option<String>,
    pub min_purchase_amount: Option<f64>,
    pub confirm_days: Option<u32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoldingConcentration {
    pub top1_weight: Option<f64>,
    pub top5_weight: Option<f64>,
    pub top10_weight: Option<f64>,
    pub hhi: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeExposure {
    pub theme: String,
    pub weight_pct: f64,
    pub source: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundAiFeatures {
    pub holding_concentration: Option<HoldingConcentration>,
    pub theme_exposures: Vec<ThemeExposure>,
}

#[derive(Debug, Deserialize)]
struct RawNavPoint {
    x: i64,
    y: f64,
    #[serde(rename = "equityReturn")]
    equity_return: Option<f64>,
}

pub struct EastmoneyFundProvider {
    client: reqwest::Client,
    base_url: String,
    f10_base_url: String,
}

impl EastmoneyFundProvider {
    pub fn new() -> Self {
        Self::with_config(DEFAULT_BASE_URL, DEFAULT_F10_BASE_URL, 15)
    }

    pub fn with_config(base_url: &str, f10_base_url: &str, timeout_secs: u64) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .user_agent("Mozilla/5.0 (compatible; Wealthfolio/1.0)")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            f10_base_url: f10_base_url.trim_end_matches('/').to_string(),
        }
    }

    pub async fn fetch_fund_snapshot(
        &self,
        code: &str,
    ) -> Result<FundResearchSnapshot, MarketDataError> {
        let code = normalize_fund_code(code)?;
        let script = self.fetch_pingzhongdata(&code).await?;
        let name = extract_js_string(&script, "fS_name").unwrap_or_else(|| code.clone());
        let nav_history = extract_nav_history(&script)?;
        let quote = nav_history.last().map(|point| FundQuote {
            nav: point.nav,
            nav_date: point.date,
            daily_return_pct: point.daily_return_pct,
        });

        let mut snapshot = FundResearchSnapshot {
            code: code.clone(),
            name,
            short_name: extract_js_string(&script, "fS_name"),
            fund_type: None,
            currency: DEFAULT_CURRENCY.to_string(),
            quote,
            nav_history,
            managers: extract_managers(&script),
            holdings: self.fetch_holdings(&code).await.unwrap_or_default(),
            sector_allocations: self.fetch_sectors(&code).await.unwrap_or_default(),
            announcements: self.fetch_announcements(&code).await.unwrap_or_default(),
            source: PROVIDER_ID.to_string(),
            fetched_at: Utc::now(),
            ..Default::default()
        };
        snapshot.risk_metrics = Some(calculate_risk_metrics(&snapshot.nav_history));
        snapshot.ai_features.holding_concentration =
            Some(calculate_holding_concentration(&snapshot.holdings));
        snapshot.holding_changes = calculate_holding_changes(&snapshot.holdings, &[]);
        Ok(snapshot)
    }

    pub async fn fetch_quote(&self, code: &str) -> Result<MarketQuote, MarketDataError> {
        let code = normalize_fund_code(code)?;
        let script = self.fetch_pingzhongdata(&code).await?;
        let latest = extract_nav_history(&script)?
            .into_iter()
            .last()
            .ok_or(MarketDataError::NoDataForRange)?;
        Ok(MarketQuote {
            symbol: code,
            price: latest.nav,
            currency: DEFAULT_CURRENCY.to_string(),
            as_of_date: latest.date,
            source: PROVIDER_ID.to_string(),
        })
    }

    pub async fn fetch_history(
        &self,
        code: &str,
        period: Period,
    ) -> Result<Vec<HistoricalPrice>, MarketDataError> {
        let code = normalize_fund_code(code)?;
        let script = self.fetch_pingzhongdata(&code).await?;
        let min_date = match period {
            Period::OneYear => Utc::now().date_naive() - chrono::Duration::days(370),
        };
        Ok(extract_nav_history(&script)?
            .into_iter()
            .filter(|point| point.date >= min_date)
            .map(|point| HistoricalPrice {
                symbol: code.clone(),
                date: point.date,
                close: point.nav,
                currency: DEFAULT_CURRENCY.to_string(),
                source: PROVIDER_ID.to_string(),
            })
            .collect())
    }

    async fn fetch_pingzhongdata(&self, code: &str) -> Result<String, MarketDataError> {
        let url = format!("{}/pingzhongdata/{}.js", self.base_url, code);
        self.get_text(&url).await
    }

    async fn get_text(&self, url: &str) -> Result<String, MarketDataError> {
        let response = self
            .client
            .get(url)
            .header(header::ACCEPT, "*/*")
            .send()
            .await
            .map_err(classify_reqwest_error)?;

        if response.status().as_u16() == 404 {
            return Err(MarketDataError::SymbolNotFound(url.to_string()));
        }
        if response.status().as_u16() == 429 {
            return Err(MarketDataError::RateLimited {
                provider: PROVIDER_ID.to_string(),
            });
        }
        if !response.status().is_success() {
            return Err(MarketDataError::ProviderError {
                provider: PROVIDER_ID.to_string(),
                message: format!("HTTP {}", response.status()),
            });
        }

        response.text().await.map_err(classify_reqwest_error)
    }

    async fn fetch_holdings(&self, code: &str) -> Result<Vec<FundHolding>, MarketDataError> {
        let url = format!(
            "{}/FundArchivesDatas.aspx?type=jjcc&code={}&topline=10",
            self.f10_base_url, code
        );
        let text = self.get_text(&url).await?;
        Ok(extract_table_holdings(&text))
    }

    async fn fetch_sectors(&self, code: &str) -> Result<Vec<SectorAllocation>, MarketDataError> {
        let url = format!(
            "{}/FundArchivesDatas.aspx?type=hypz&code={}",
            self.f10_base_url, code
        );
        let text = self.get_text(&url).await?;
        Ok(extract_sector_allocations(&text))
    }

    async fn fetch_announcements(
        &self,
        code: &str,
    ) -> Result<Vec<FundAnnouncement>, MarketDataError> {
        let url = format!("{}/jjgg_{}.html", self.f10_base_url, code);
        let text = self.get_text(&url).await?;
        Ok(extract_announcements(&text))
    }

    fn extract_code(instrument: &ProviderInstrument) -> Result<String, MarketDataError> {
        match instrument {
            ProviderInstrument::FundCode { code } => normalize_fund_code(code),
            ProviderInstrument::EquitySymbol { symbol } => normalize_fund_code(symbol),
            _ => Err(MarketDataError::UnsupportedAssetType(
                "Eastmoney fund provider requires a 6-digit fund code".to_string(),
            )),
        }
    }
}

impl Default for EastmoneyFundProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MarketDataProvider for EastmoneyFundProvider {
    fn id(&self) -> &'static str {
        PROVIDER_ID
    }

    fn priority(&self) -> u8 {
        2
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            instrument_kinds: &[InstrumentKind::Fund],
            coverage: Coverage::global_best_effort(),
            supports_latest: true,
            supports_historical: true,
            supports_search: false,
            supports_profile: false,
            supports_dividends: false,
        }
    }

    fn rate_limit(&self) -> RateLimit {
        RateLimit {
            requests_per_minute: 30,
            max_concurrency: 2,
            min_delay: Duration::from_millis(250),
        }
    }

    async fn get_latest_quote(
        &self,
        _context: &QuoteContext,
        instrument: ProviderInstrument,
    ) -> Result<Quote, MarketDataError> {
        let quote = self.fetch_quote(&Self::extract_code(&instrument)?).await?;
        Ok(Quote::new(
            date_to_utc(quote.as_of_date),
            decimal_from_f64(quote.price)?,
            quote.currency,
            PROVIDER_ID.to_string(),
        ))
    }

    async fn get_historical_quotes(
        &self,
        _context: &QuoteContext,
        instrument: ProviderInstrument,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Quote>, MarketDataError> {
        let history = self
            .fetch_history(&Self::extract_code(&instrument)?, Period::OneYear)
            .await?;
        Ok(history
            .into_iter()
            .filter(|price| {
                let timestamp = date_to_utc(price.date);
                timestamp >= start && timestamp <= end
            })
            .map(|price| {
                Ok(Quote::new(
                    date_to_utc(price.date),
                    decimal_from_f64(price.close)?,
                    price.currency,
                    PROVIDER_ID.to_string(),
                ))
            })
            .collect::<Result<Vec<_>, MarketDataError>>()?)
    }
}

fn normalize_fund_code(code: &str) -> Result<String, MarketDataError> {
    let code = code.trim();
    if code.len() == 6 && code.chars().all(|ch| ch.is_ascii_digit()) {
        Ok(code.to_string())
    } else {
        Err(MarketDataError::SymbolNotFound(code.to_string()))
    }
}

fn classify_reqwest_error(error: reqwest::Error) -> MarketDataError {
    if error.is_timeout() {
        MarketDataError::Timeout {
            provider: PROVIDER_ID.to_string(),
        }
    } else {
        MarketDataError::Network(error)
    }
}

fn decimal_from_f64(value: f64) -> Result<Decimal, MarketDataError> {
    Decimal::from_f64(value).ok_or_else(|| MarketDataError::ValidationFailed {
        message: format!("Invalid decimal value: {value}"),
    })
}

fn date_to_utc(date: NaiveDate) -> DateTime<Utc> {
    Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0).expect("valid midnight"))
}

fn extract_js_string(script: &str, name: &str) -> Option<String> {
    let marker = format!("var {name} = \"");
    let start = script.find(&marker)? + marker.len();
    let end = script[start..].find('"')? + start;
    Some(script[start..end].to_string())
}

fn extract_js_array<'a>(script: &'a str, name: &str) -> Option<&'a str> {
    let marker = format!("var {name} = ");
    let start = script.find(&marker)? + marker.len();
    let after = &script[start..];
    let end = after.find(";\n").or_else(|| after.find(';'))?;
    Some(after[..end].trim())
}

fn extract_nav_history(script: &str) -> Result<Vec<FundNavPoint>, MarketDataError> {
    let raw = extract_js_array(script, "Data_netWorthTrend").ok_or_else(|| {
        MarketDataError::ProviderError {
            provider: PROVIDER_ID.to_string(),
            message: "Missing Data_netWorthTrend".to_string(),
        }
    })?;
    let points: Vec<RawNavPoint> =
        serde_json::from_str(raw).map_err(|error| MarketDataError::ProviderError {
            provider: PROVIDER_ID.to_string(),
            message: format!("Invalid NAV history: {error}"),
        })?;

    let history = points
        .into_iter()
        .filter_map(|point| {
            Utc.timestamp_millis_opt(point.x)
                .single()
                .map(|timestamp| FundNavPoint {
                    date: timestamp.date_naive(),
                    nav: point.y,
                    daily_return_pct: point.equity_return,
                })
        })
        .collect::<Vec<_>>();

    if history.is_empty() {
        Err(MarketDataError::NoDataForRange)
    } else {
        Ok(history)
    }
}

fn extract_managers(script: &str) -> Vec<FundManager> {
    let Some(raw) = extract_js_array(script, "Data_currentFundManager") else {
        return vec![];
    };
    let Ok(values) = serde_json::from_str::<Vec<serde_json::Value>>(raw) else {
        return vec![];
    };
    values
        .into_iter()
        .filter_map(|value| {
            let name = value.get("name")?.as_str()?.trim();
            if name.is_empty() {
                return None;
            }
            Some(FundManager {
                name: name.to_string(),
                started_on: value
                    .get("workTime")
                    .and_then(serde_json::Value::as_str)
                    .and_then(parse_date),
                ..Default::default()
            })
        })
        .collect()
}

fn calculate_risk_metrics(history: &[FundNavPoint]) -> FundRiskMetrics {
    let returns = history
        .windows(2)
        .filter_map(|pair| {
            let previous = pair[0].nav;
            (previous > 0.0).then_some(pair[1].nav / previous - 1.0)
        })
        .collect::<Vec<_>>();

    let annualized_volatility_pct = stddev(&returns).map(|v| v * 252.0_f64.sqrt() * 100.0);
    let downside = returns
        .iter()
        .copied()
        .filter(|value| *value < 0.0)
        .collect::<Vec<_>>();
    let downside_volatility_pct = stddev(&downside).map(|v| v * 252.0_f64.sqrt() * 100.0);
    let max_drawdown_pct = max_drawdown(history);
    let annualized_return = annualized_return(history);
    let calmar_ratio = annualized_return
        .zip(max_drawdown_pct)
        .and_then(|(ret, dd)| {
            let abs_dd = dd.abs() / 100.0;
            (abs_dd > 0.0).then_some(ret / abs_dd)
        });

    FundRiskMetrics {
        max_drawdown_pct,
        max_drawdown_1y_pct: max_drawdown_pct,
        annualized_volatility_pct,
        downside_volatility_pct,
        sharpe_ratio: annualized_return
            .zip(stddev(&returns))
            .and_then(|(ret, vol)| {
                let annual_vol = vol * 252.0_f64.sqrt();
                (annual_vol > 0.0).then_some(ret / annual_vol)
            }),
        return_drawdown_ratio: annualized_return
            .zip(max_drawdown_pct)
            .and_then(|(ret, dd)| {
                let abs_dd = dd.abs() / 100.0;
                (abs_dd > 0.0).then_some(ret / abs_dd)
            }),
        calmar_ratio,
        ..Default::default()
    }
}

fn stddev(values: &[f64]) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (values.len() - 1) as f64;
    Some(variance.sqrt())
}

fn max_drawdown(history: &[FundNavPoint]) -> Option<f64> {
    let mut peak = 0.0;
    let mut worst = 0.0;
    for point in history {
        if point.nav > peak {
            peak = point.nav;
        }
        if peak > 0.0 {
            let drawdown = (point.nav - peak) / peak;
            if drawdown < worst {
                worst = drawdown;
            }
        }
    }
    Some(worst * 100.0)
}

fn annualized_return(history: &[FundNavPoint]) -> Option<f64> {
    let first = history.first()?;
    let last = history.last()?;
    let days = (last.date - first.date).num_days();
    if first.nav <= 0.0 || days <= 0 {
        return None;
    }
    Some((last.nav / first.nav).powf(365.0 / days as f64) - 1.0)
}

fn calculate_holding_concentration(holdings: &[FundHolding]) -> HoldingConcentration {
    let weights = holdings
        .iter()
        .filter_map(|holding| holding.weight_pct)
        .collect::<Vec<_>>();
    HoldingConcentration {
        top1_weight: weights.first().copied(),
        top5_weight: sum_first(&weights, 5),
        top10_weight: sum_first(&weights, 10),
        hhi: (!weights.is_empty()).then(|| weights.iter().map(|w| (w / 100.0).powi(2)).sum()),
    }
}

fn sum_first(values: &[f64], count: usize) -> Option<f64> {
    (!values.is_empty()).then(|| values.iter().take(count).sum())
}

fn calculate_holding_changes(
    current: &[FundHolding],
    previous: &[FundHolding],
) -> Vec<FundHoldingChange> {
    current
        .iter()
        .map(|holding| {
            let prev = previous.iter().find(|item| {
                item.stock_code == holding.stock_code || item.stock_name == holding.stock_name
            });
            let previous_weight_pct = prev.and_then(|item| item.weight_pct);
            let weight_change_pct = holding
                .weight_pct
                .zip(previous_weight_pct)
                .map(|(current, previous)| current - previous);
            let change_type = match weight_change_pct {
                None if prev.is_none() => HoldingChangeType::New,
                Some(delta) if delta > 0.0 => HoldingChangeType::Increased,
                Some(delta) if delta < 0.0 => HoldingChangeType::Decreased,
                _ => HoldingChangeType::Unchanged,
            };
            FundHoldingChange {
                stock_code: holding.stock_code.clone(),
                stock_name: holding.stock_name.clone(),
                previous_weight_pct,
                current_weight_pct: holding.weight_pct,
                weight_change_pct,
                previous_rank: prev.and_then(|item| item.rank),
                current_rank: holding.rank,
                rank_change: holding.rank.zip(prev.and_then(|item| item.rank)).map(
                    |(current_rank, previous_rank)| previous_rank as i32 - current_rank as i32,
                ),
                change_type,
            }
        })
        .collect()
}

fn extract_table_holdings(text: &str) -> Vec<FundHolding> {
    extract_rows(text)
        .into_iter()
        .filter_map(|row| {
            let cells = extract_cells(&row);
            let stock_name = cells.iter().find(|cell| has_cjk_or_alpha(cell))?.trim();
            Some(FundHolding {
                stock_code: cells.iter().find_map(|cell| find_six_digit(cell)),
                stock_name: stock_name.to_string(),
                weight_pct: cells.iter().find_map(|cell| parse_percent(cell)),
                rank: cells
                    .iter()
                    .find_map(|cell| cell.trim().parse::<u32>().ok()),
                report_date: cells.iter().find_map(|cell| parse_date(cell)),
            })
        })
        .take(10)
        .collect()
}

fn extract_sector_allocations(text: &str) -> Vec<SectorAllocation> {
    extract_rows(text)
        .into_iter()
        .filter_map(|row| {
            let cells = extract_cells(&row);
            let sector = cells.iter().find(|cell| has_cjk_or_alpha(cell))?.trim();
            Some(SectorAllocation {
                sector: sector.to_string(),
                weight_pct: cells.iter().find_map(|cell| parse_percent(cell)),
                report_date: cells.iter().find_map(|cell| parse_date(cell)),
                ..Default::default()
            })
        })
        .collect()
}

fn extract_announcements(text: &str) -> Vec<FundAnnouncement> {
    extract_anchor_like_items(text)
        .into_iter()
        .filter_map(|(title, url)| {
            let title = html_unescape(strip_tags(&title)).trim().to_string();
            if title.is_empty() {
                return None;
            }
            let category = classify_announcement(&title);
            let importance = announcement_importance(&category);
            Some(FundAnnouncement {
                date: parse_date(&title),
                title,
                url,
                category,
                importance,
            })
        })
        .take(50)
        .collect()
}

fn classify_announcement(title: &str) -> Option<AnnouncementCategory> {
    let category = if title.contains("季度") {
        AnnouncementCategory::QuarterlyReport
    } else if title.contains("半年") {
        AnnouncementCategory::SemiAnnualReport
    } else if title.contains("年度") || title.contains("年报") {
        AnnouncementCategory::AnnualReport
    } else if title.contains("基金经理") {
        AnnouncementCategory::ManagerChange
    } else if title.contains("合同") {
        AnnouncementCategory::ContractChange
    } else if title.contains("分红") {
        AnnouncementCategory::Dividend
    } else if title.contains("暂停申购") || title.contains("暂停赎回") {
        AnnouncementCategory::PurchaseSuspension
    } else if title.contains("恢复申购") || title.contains("恢复赎回") {
        AnnouncementCategory::PurchaseResume
    } else if title.contains("风险") {
        AnnouncementCategory::RiskWarning
    } else if title.contains("清盘") {
        AnnouncementCategory::LiquidationRisk
    } else if title.contains("重大") {
        AnnouncementCategory::MajorEvent
    } else {
        AnnouncementCategory::Other
    };
    Some(category)
}

fn announcement_importance(category: &Option<AnnouncementCategory>) -> AnnouncementImportance {
    match category {
        Some(
            AnnouncementCategory::ManagerChange
            | AnnouncementCategory::PurchaseSuspension
            | AnnouncementCategory::RiskWarning
            | AnnouncementCategory::LiquidationRisk
            | AnnouncementCategory::MajorEvent,
        ) => AnnouncementImportance::High,
        Some(
            AnnouncementCategory::QuarterlyReport
            | AnnouncementCategory::SemiAnnualReport
            | AnnouncementCategory::AnnualReport
            | AnnouncementCategory::Dividend,
        ) => AnnouncementImportance::Medium,
        _ => AnnouncementImportance::Low,
    }
}

fn extract_rows(text: &str) -> Vec<String> {
    text.split("<tr")
        .skip(1)
        .filter_map(|part| part.split_once("</tr>").map(|(row, _)| row.to_string()))
        .collect()
}

fn extract_cells(row: &str) -> Vec<String> {
    row.split("<td")
        .skip(1)
        .filter_map(|part| part.split_once("</td>").map(|(cell, _)| strip_tags(cell)))
        .map(|cell| html_unescape(cell).trim().to_string())
        .filter(|cell| !cell.is_empty())
        .collect()
}

fn strip_tags(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut in_tag = false;
    for ch in value.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }
    output
}

fn html_unescape(value: String) -> String {
    value
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
}

fn parse_percent(value: &str) -> Option<f64> {
    value
        .trim()
        .trim_end_matches('%')
        .replace(',', "")
        .parse::<f64>()
        .ok()
}

fn parse_date(value: &str) -> Option<NaiveDate> {
    for token in value.split(|ch: char| !ch.is_ascii_digit() && ch != '-') {
        if token.len() == 10 {
            if let Ok(date) = NaiveDate::parse_from_str(token, "%Y-%m-%d") {
                return Some(date);
            }
        }
    }
    None
}

fn find_six_digit(value: &str) -> Option<String> {
    value
        .split(|ch: char| !ch.is_ascii_digit())
        .find(|part| part.len() == 6)
        .map(str::to_string)
}

fn has_cjk_or_alpha(value: &str) -> bool {
    value
        .chars()
        .any(|ch| ch.is_alphabetic() || ('\u{4e00}'..='\u{9fff}').contains(&ch))
}

fn extract_anchor_like_items(text: &str) -> Vec<(String, Option<String>)> {
    text.split("<a")
        .skip(1)
        .filter_map(|part| {
            let (attrs, rest) = part.split_once('>')?;
            let (label, _) = rest.split_once("</a>")?;
            Some((label.to_string(), extract_href(attrs)))
        })
        .collect()
}

fn extract_href(attrs: &str) -> Option<String> {
    let marker = "href=\"";
    let start = attrs.find(marker)? + marker.len();
    let end = attrs[start..].find('"')? + start;
    Some(attrs[start..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pingzhongdata_nav_history() {
        let script = r#"
var fS_name = "测试基金";
var Data_netWorthTrend = [{"x":1704067200000,"y":1.2345,"equityReturn":0.12}];
var Data_currentFundManager = [{"name":"张三","workTime":"2024-01-01"}];
"#;
        let history = extract_nav_history(script).unwrap();
        assert_eq!(
            history[0].date,
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
        );
        assert_eq!(history[0].nav, 1.2345);
        assert_eq!(extract_managers(script)[0].name, "张三");
    }

    #[test]
    fn rejects_non_fund_codes() {
        assert!(normalize_fund_code("014002").is_ok());
        assert!(normalize_fund_code("AAPL").is_err());
    }
}
