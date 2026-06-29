//! Eastmoney stock profile provider.
//!
//! Uses Eastmoney quote endpoints for stock metadata needed by profile
//! enrichment and fund-holding lookthrough.

use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::header;
use serde::{Deserialize, Serialize};

use crate::errors::MarketDataError;
use crate::models::{
    AssetProfile, Coverage, InstrumentKind, ProviderInstrument, Quote, QuoteContext,
};
use crate::provider::{MarketDataProvider, ProviderCapabilities, RateLimit};

const PROVIDER_ID: &str = "EASTMONEY_STOCK";
const DEFAULT_QUOTE_BASE_URL: &str = "https://quote.eastmoney.com";
const DEFAULT_API_BASE_URL: &str = "https://push2.eastmoney.com";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EastmoneyStockProfile {
    pub symbol: String,
    pub name: Option<String>,
    pub market: EastmoneyStockMarket,
    pub quote_url: String,
    pub secid: String,
    pub f10_code: Option<String>,
    pub industry: Option<String>,
    pub region: Option<String>,
    pub concepts: Vec<String>,
    pub market_cap: Option<f64>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EastmoneyStockMarket {
    Shanghai,
    Star,
    Shenzhen,
    Beijing,
    HongKong,
    UnitedStates,
}

impl EastmoneyStockMarket {
    pub fn as_label(self) -> &'static str {
        match self {
            Self::Shanghai => "A-share Shanghai",
            Self::Star => "A-share STAR",
            Self::Shenzhen => "A-share Shenzhen",
            Self::Beijing => "Beijing Stock Exchange",
            Self::HongKong => "Hong Kong",
            Self::UnitedStates => "United States",
        }
    }
}

#[derive(Clone, Debug)]
pub struct EastmoneyStockRoute {
    pub symbol: String,
    pub market: EastmoneyStockMarket,
    pub quote_url: String,
    pub secid: String,
    pub f10_code: Option<String>,
}

#[derive(Clone, Debug)]
pub struct EastmoneyStockProvider {
    client: reqwest::Client,
    quote_base_url: String,
    api_base_url: String,
}

impl EastmoneyStockProvider {
    pub fn new() -> Self {
        Self::with_config(DEFAULT_QUOTE_BASE_URL, DEFAULT_API_BASE_URL, 15)
    }

    pub fn with_config(quote_base_url: &str, api_base_url: &str, timeout_secs: u64) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .user_agent("Mozilla/5.0 (compatible; Wealthfolio/1.0)")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            client,
            quote_base_url: quote_base_url.trim_end_matches('/').to_string(),
            api_base_url: api_base_url.trim_end_matches('/').to_string(),
        }
    }

    pub async fn fetch_stock_profile(
        &self,
        symbol: &str,
    ) -> Result<EastmoneyStockProfile, MarketDataError> {
        let route = self.resolve_route(symbol)?;
        let quote = self.fetch_quote_profile(&route.secid).await?;
        Ok(EastmoneyStockProfile {
            symbol: route.symbol,
            name: clean_text(quote.name),
            market: route.market,
            quote_url: route.quote_url,
            secid: route.secid,
            f10_code: route.f10_code,
            industry: clean_text(quote.industry),
            region: clean_text(quote.region),
            concepts: split_concepts(quote.concepts),
            market_cap: quote.market_cap,
        })
    }

    pub fn resolve_route(&self, symbol: &str) -> Result<EastmoneyStockRoute, MarketDataError> {
        resolve_route(symbol, &self.quote_base_url)
            .ok_or_else(|| MarketDataError::SymbolNotFound(symbol.trim().to_string()))
    }

    async fn fetch_quote_profile(
        &self,
        secid: &str,
    ) -> Result<EastmoneyQuoteProfile, MarketDataError> {
        let url = format!(
            "{}/api/qt/stock/get?secid={}&fields=f57,f58,f127,f128,f129,f116",
            self.api_base_url, secid
        );
        let response = self
            .client
            .get(&url)
            .header(header::ACCEPT, "application/json,text/plain,*/*")
            .send()
            .await
            .map_err(classify_reqwest_error)?;

        if response.status().as_u16() == 404 {
            return Err(MarketDataError::SymbolNotFound(secid.to_string()));
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

        let payload = response.text().await.map_err(classify_reqwest_error)?;
        let decoded: EastmoneyQuoteResponse =
            serde_json::from_str(&payload).map_err(|error| MarketDataError::ProviderError {
                provider: PROVIDER_ID.to_string(),
                message: format!("Invalid quote profile: {error}"),
            })?;
        decoded
            .data
            .ok_or_else(|| MarketDataError::SymbolNotFound(secid.to_string()))
    }
}

impl Default for EastmoneyStockProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MarketDataProvider for EastmoneyStockProvider {
    fn id(&self) -> &'static str {
        PROVIDER_ID
    }

    fn priority(&self) -> u8 {
        8
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            instrument_kinds: &[InstrumentKind::Equity],
            coverage: Coverage::global_best_effort(),
            supports_latest: false,
            supports_historical: false,
            supports_search: false,
            supports_profile: true,
            supports_dividends: false,
        }
    }

    fn rate_limit(&self) -> RateLimit {
        RateLimit {
            requests_per_minute: 60,
            max_concurrency: 2,
            min_delay: Duration::from_millis(200),
        }
    }

    async fn get_latest_quote(
        &self,
        _context: &QuoteContext,
        _instrument: ProviderInstrument,
    ) -> Result<Quote, MarketDataError> {
        Err(MarketDataError::NotSupported {
            operation: "get_latest_quote".to_string(),
            provider: PROVIDER_ID.to_string(),
        })
    }

    async fn get_historical_quotes(
        &self,
        _context: &QuoteContext,
        _instrument: ProviderInstrument,
        _start: DateTime<Utc>,
        _end: DateTime<Utc>,
    ) -> Result<Vec<Quote>, MarketDataError> {
        Err(MarketDataError::NotSupported {
            operation: "get_historical_quotes".to_string(),
            provider: PROVIDER_ID.to_string(),
        })
    }

    async fn get_profile(&self, symbol: &str) -> Result<AssetProfile, MarketDataError> {
        let profile = self.fetch_stock_profile(symbol).await?;
        Ok(AssetProfile {
            source: Some(PROVIDER_ID.to_string()),
            name: profile.name,
            quote_type: Some("EQUITY".to_string()),
            sector: profile.industry.clone(),
            sectors: None,
            industry: profile.industry,
            website: Some(profile.quote_url),
            market_cap: profile.market_cap,
            country: country_for_market(profile.market).map(str::to_string),
            ..Default::default()
        })
    }
}

#[derive(Debug, Deserialize)]
struct EastmoneyQuoteResponse {
    data: Option<EastmoneyQuoteProfile>,
}

#[derive(Debug, Deserialize)]
struct EastmoneyQuoteProfile {
    #[serde(rename = "f58")]
    name: Option<String>,
    #[serde(rename = "f127")]
    industry: Option<String>,
    #[serde(rename = "f128")]
    region: Option<String>,
    #[serde(rename = "f129")]
    concepts: Option<String>,
    #[serde(rename = "f116")]
    market_cap: Option<f64>,
}

fn resolve_route(symbol: &str, quote_base_url: &str) -> Option<EastmoneyStockRoute> {
    let normalized = normalize_symbol(symbol)?;
    let (market, plain_symbol) = detect_market(&normalized)?;
    let quote_url = match market {
        EastmoneyStockMarket::Shanghai => format!("{quote_base_url}/sh{plain_symbol}.html"),
        EastmoneyStockMarket::Star => format!("{quote_base_url}/kcb/{plain_symbol}.html"),
        EastmoneyStockMarket::Shenzhen => format!("{quote_base_url}/sz{plain_symbol}.html"),
        EastmoneyStockMarket::Beijing => format!("{quote_base_url}/bj{plain_symbol}.html"),
        EastmoneyStockMarket::HongKong => format!("{quote_base_url}/hk/{plain_symbol}.html"),
        EastmoneyStockMarket::UnitedStates => format!("{quote_base_url}/us/{plain_symbol}.html"),
    };
    let secid = match market {
        EastmoneyStockMarket::Shanghai | EastmoneyStockMarket::Star => {
            format!("1.{plain_symbol}")
        }
        EastmoneyStockMarket::Shenzhen | EastmoneyStockMarket::Beijing => {
            format!("0.{plain_symbol}")
        }
        EastmoneyStockMarket::HongKong => format!("116.{plain_symbol}"),
        EastmoneyStockMarket::UnitedStates => format!("105.{plain_symbol}"),
    };
    let f10_code = match market {
        EastmoneyStockMarket::Shanghai | EastmoneyStockMarket::Star => {
            Some(format!("SH{plain_symbol}"))
        }
        EastmoneyStockMarket::Shenzhen => Some(format!("SZ{plain_symbol}")),
        EastmoneyStockMarket::Beijing => Some(format!("BJ{plain_symbol}")),
        EastmoneyStockMarket::HongKong => Some(plain_symbol.clone()),
        EastmoneyStockMarket::UnitedStates => Some(plain_symbol.clone()),
    };

    Some(EastmoneyStockRoute {
        symbol: plain_symbol,
        market,
        quote_url,
        secid,
        f10_code,
    })
}

fn normalize_symbol(symbol: &str) -> Option<String> {
    let trimmed = symbol.trim();
    if trimmed.is_empty() {
        return None;
    }
    let without_url = trimmed
        .rsplit('/')
        .next()
        .unwrap_or(trimmed)
        .trim_end_matches(".html");
    let without_secid = without_url
        .strip_prefix("1.")
        .or_else(|| without_url.strip_prefix("0."))
        .or_else(|| without_url.strip_prefix("105."))
        .or_else(|| without_url.strip_prefix("116."))
        .unwrap_or(without_url);
    Some(without_secid.to_ascii_uppercase())
}

fn detect_market(symbol: &str) -> Option<(EastmoneyStockMarket, String)> {
    if let Some(code) = symbol.strip_prefix("SH") {
        return detect_numeric_market(code, Some(EastmoneyStockMarket::Shanghai));
    }
    if let Some(code) = symbol.strip_prefix("SZ") {
        return detect_numeric_market(code, Some(EastmoneyStockMarket::Shenzhen));
    }
    if let Some(code) = symbol.strip_prefix("BJ") {
        return detect_numeric_market(code, Some(EastmoneyStockMarket::Beijing));
    }
    if let Some(code) = symbol.strip_prefix("HK") {
        return Some((EastmoneyStockMarket::HongKong, pad_hk_code(code)?));
    }
    if symbol.chars().all(|ch| ch.is_ascii_digit()) {
        if symbol.len() <= 5 {
            return Some((EastmoneyStockMarket::HongKong, pad_hk_code(symbol)?));
        }
        return detect_numeric_market(symbol, None);
    }
    if symbol
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '-')
    {
        return Some((EastmoneyStockMarket::UnitedStates, symbol.to_string()));
    }
    None
}

fn detect_numeric_market(
    code: &str,
    explicit_market: Option<EastmoneyStockMarket>,
) -> Option<(EastmoneyStockMarket, String)> {
    if code.len() != 6 || !code.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    let market = explicit_market.unwrap_or_else(|| {
        if code.starts_with("688") || code.starts_with("689") {
            EastmoneyStockMarket::Star
        } else if code.starts_with('6') {
            EastmoneyStockMarket::Shanghai
        } else if code.starts_with('8') || code.starts_with('9') || code.starts_with('4') {
            EastmoneyStockMarket::Beijing
        } else {
            EastmoneyStockMarket::Shenzhen
        }
    });
    Some((market, code.to_string()))
}

fn pad_hk_code(code: &str) -> Option<String> {
    if code.is_empty() || code.len() > 5 || !code.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    Some(format!("{:0>5}", code))
}

fn split_concepts(concepts: Option<String>) -> Vec<String> {
    concepts
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn clean_text(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty() && item != "-" && item != "--")
}

fn country_for_market(market: EastmoneyStockMarket) -> Option<&'static str> {
    match market {
        EastmoneyStockMarket::Shanghai
        | EastmoneyStockMarket::Star
        | EastmoneyStockMarket::Shenzhen
        | EastmoneyStockMarket::Beijing => Some("CN"),
        EastmoneyStockMarket::HongKong => Some("HK"),
        EastmoneyStockMarket::UnitedStates => Some("US"),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_mainland_quote_routes() {
        let provider = EastmoneyStockProvider::new();

        let sh = provider.resolve_route("600397").unwrap();
        assert_eq!(sh.market, EastmoneyStockMarket::Shanghai);
        assert_eq!(sh.secid, "1.600397");
        assert_eq!(sh.quote_url, "https://quote.eastmoney.com/sh600397.html");
        assert_eq!(sh.f10_code.as_deref(), Some("SH600397"));

        let star = provider.resolve_route("688708").unwrap();
        assert_eq!(star.market, EastmoneyStockMarket::Star);
        assert_eq!(star.secid, "1.688708");
        assert_eq!(
            star.quote_url,
            "https://quote.eastmoney.com/kcb/688708.html"
        );
        assert_eq!(star.f10_code.as_deref(), Some("SH688708"));

        let sz = provider.resolve_route("sz300059").unwrap();
        assert_eq!(sz.market, EastmoneyStockMarket::Shenzhen);
        assert_eq!(sz.secid, "0.300059");
        assert_eq!(sz.quote_url, "https://quote.eastmoney.com/sz300059.html");
    }

    #[test]
    fn resolves_global_quote_routes() {
        let provider = EastmoneyStockProvider::new();

        let us = provider.resolve_route("MU").unwrap();
        assert_eq!(us.market, EastmoneyStockMarket::UnitedStates);
        assert_eq!(us.secid, "105.MU");
        assert_eq!(us.quote_url, "https://quote.eastmoney.com/us/MU.html");

        let hk = provider.resolve_route("981").unwrap();
        assert_eq!(hk.market, EastmoneyStockMarket::HongKong);
        assert_eq!(hk.secid, "116.00981");
        assert_eq!(hk.quote_url, "https://quote.eastmoney.com/hk/00981.html");
    }

    #[test]
    fn splits_concepts() {
        assert_eq!(
            split_concepts(Some("军工, 新材料,,人工智能".to_string())),
            vec!["军工", "新材料", "人工智能"]
        );
    }
}
