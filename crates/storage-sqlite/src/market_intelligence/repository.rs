use std::collections::BTreeSet;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sql_query;
use diesel::sql_types::{Double, Integer, Nullable, Text};
use diesel::sqlite::SqliteConnection;
use sha2::{Digest, Sha256};

use crate::db::{get_connection, WriteHandle};
use crate::errors::StorageError;
use wealthfolio_core::errors::{Error, Result};
use wealthfolio_core::market_intelligence::{
    CapitalFlowRepository, CapitalFlowSnapshot, MacroCapitalRepository, MacroCapitalSnapshot,
    MarketIntelligenceIntradayRepository, MarketIntelligenceIntradaySnapshot, MarketSnapshot,
    MarketSnapshotRepository, PortfolioThemeExposure, PortfolioThemeExposureRepository,
    SectorRotationRepository, SectorRotationSnapshot, ThemeRotationRepository,
    ThemeRotationSnapshot,
};

pub struct MarketIntelligenceSqliteRepository {
    pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
    writer: WriteHandle,
}

impl MarketIntelligenceSqliteRepository {
    pub fn new(pool: Arc<Pool<ConnectionManager<SqliteConnection>>>, writer: WriteHandle) -> Self {
        Self { pool, writer }
    }
}

#[derive(QueryableByName)]
struct MarketSnapshotRow {
    #[diesel(sql_type = Text)]
    market: String,
    #[diesel(sql_type = Text)]
    index_name: String,
    #[diesel(sql_type = Double)]
    price: f64,
    #[diesel(sql_type = Double)]
    change_pct: f64,
    #[diesel(sql_type = Nullable<Double>)]
    turnover: Option<f64>,
    #[diesel(sql_type = Text)]
    timestamp: String,
    #[diesel(sql_type = Text)]
    source: String,
}

#[derive(QueryableByName)]
struct CapitalFlowRow {
    #[diesel(sql_type = Text)]
    market: String,
    #[diesel(sql_type = Text)]
    date: String,
    #[diesel(sql_type = Text)]
    category: String,
    #[diesel(sql_type = Nullable<Double>)]
    inflow: Option<f64>,
    #[diesel(sql_type = Nullable<Double>)]
    outflow: Option<f64>,
    #[diesel(sql_type = Double)]
    net_flow: f64,
    #[diesel(sql_type = Text)]
    source: String,
}

#[derive(QueryableByName)]
struct SectorRotationRow {
    #[diesel(sql_type = Text)]
    market: String,
    #[diesel(sql_type = Text)]
    sector: String,
    #[diesel(sql_type = Text)]
    date: String,
    #[diesel(sql_type = Nullable<Double>)]
    net_flow: Option<f64>,
    #[diesel(sql_type = Nullable<Double>)]
    change_pct: Option<f64>,
    #[diesel(sql_type = Nullable<Double>)]
    turnover: Option<f64>,
    #[diesel(sql_type = Nullable<Integer>)]
    ranking: Option<i32>,
    #[diesel(sql_type = Text)]
    source: String,
}

#[derive(QueryableByName)]
struct ThemeRotationRow {
    #[diesel(sql_type = Text)]
    theme: String,
    #[diesel(sql_type = Text)]
    date: String,
    #[diesel(sql_type = Nullable<Double>)]
    flow_score: Option<f64>,
    #[diesel(sql_type = Nullable<Double>)]
    momentum: Option<f64>,
    #[diesel(sql_type = Nullable<Integer>)]
    ranking: Option<i32>,
}

#[derive(QueryableByName)]
struct PortfolioThemeExposureRow {
    #[diesel(sql_type = Text)]
    portfolio_id: String,
    #[diesel(sql_type = Text)]
    theme: String,
    #[diesel(sql_type = Double)]
    weight_pct: f64,
    #[diesel(sql_type = Double)]
    market_value: f64,
    #[diesel(sql_type = Text)]
    source: String,
    #[diesel(sql_type = Text)]
    timestamp: String,
}

#[derive(QueryableByName)]
struct IntradaySnapshotRow {
    #[diesel(sql_type = Text)]
    market: String,
    #[diesel(sql_type = Text)]
    kind: String,
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Text)]
    timestamp: String,
    #[diesel(sql_type = Nullable<Double>)]
    open: Option<f64>,
    #[diesel(sql_type = Nullable<Double>)]
    close: Option<f64>,
    #[diesel(sql_type = Nullable<Double>)]
    high: Option<f64>,
    #[diesel(sql_type = Nullable<Double>)]
    low: Option<f64>,
    #[diesel(sql_type = Nullable<Double>)]
    volume: Option<f64>,
    #[diesel(sql_type = Nullable<Double>)]
    amount: Option<f64>,
    #[diesel(sql_type = Nullable<Double>)]
    net_flow: Option<f64>,
    #[diesel(sql_type = Nullable<Double>)]
    change_pct: Option<f64>,
    #[diesel(sql_type = Nullable<Double>)]
    turnover: Option<f64>,
    #[diesel(sql_type = Nullable<Integer>)]
    ranking: Option<i32>,
    #[diesel(sql_type = Text)]
    source: String,
}

#[derive(QueryableByName)]
struct MacroCapitalRow {
    #[diesel(sql_type = Text)]
    indicator: String,
    #[diesel(sql_type = Text)]
    market: String,
    #[diesel(sql_type = Text)]
    date: String,
    #[diesel(sql_type = Double)]
    value: f64,
    #[diesel(sql_type = Nullable<Double>)]
    change: Option<f64>,
    #[diesel(sql_type = Nullable<Text>)]
    unit: Option<String>,
    #[diesel(sql_type = Text)]
    source: String,
}

#[async_trait]
impl MarketSnapshotRepository for MarketIntelligenceSqliteRepository {
    async fn save_market_snapshots(&self, snapshots: &[MarketSnapshot]) -> Result<()> {
        let snapshots = snapshots.to_vec();
        self.writer
            .exec_tx(move |tx| {
                for snapshot in &snapshots {
                    sql_query(
                        "INSERT OR IGNORE INTO market_snapshots
                         (id, market, index_name, price, change_pct, turnover, timestamp, source)
                         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                    )
                    .bind::<Text, _>(stable_id(
                        "market_snapshot",
                        &[
                            &snapshot.market,
                            &snapshot.index_name,
                            &snapshot.timestamp.to_rfc3339(),
                            &snapshot.source,
                        ],
                    ))
                    .bind::<Text, _>(&snapshot.market)
                    .bind::<Text, _>(&snapshot.index_name)
                    .bind::<Double, _>(snapshot.price)
                    .bind::<Double, _>(snapshot.change_pct)
                    .bind::<Nullable<Double>, _>(snapshot.turnover)
                    .bind::<Text, _>(snapshot.timestamp.to_rfc3339())
                    .bind::<Text, _>(&snapshot.source)
                    .execute(tx.conn())
                    .map_err(StorageError::QueryFailed)?;
                }
                Ok(())
            })
            .await
    }

    async fn latest_market_snapshots(&self, market: Option<&str>) -> Result<Vec<MarketSnapshot>> {
        let mut conn = get_connection(&self.pool)?;
        let rows = if let Some(market) = market {
            sql_query(
                "SELECT market, index_name, price, change_pct, turnover, timestamp, source
                 FROM market_snapshots
                 WHERE market = ?
                 ORDER BY timestamp DESC",
            )
            .bind::<Text, _>(market)
            .load::<MarketSnapshotRow>(&mut conn)
        } else {
            sql_query(
                "SELECT market, index_name, price, change_pct, turnover, timestamp, source
                 FROM market_snapshots
                 ORDER BY timestamp DESC",
            )
            .load::<MarketSnapshotRow>(&mut conn)
        }
        .map_err(StorageError::QueryFailed)?;
        latest_by_key(rows, |row| (row.market.clone(), row.index_name.clone()))
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }

    async fn market_snapshot_history(
        &self,
        market: &str,
        index_name: &str,
        limit: usize,
    ) -> Result<Vec<MarketSnapshot>> {
        let mut conn = get_connection(&self.pool)?;
        let rows = sql_query(
            "SELECT market, index_name, price, change_pct, turnover, timestamp, source
             FROM market_snapshots
             WHERE market = ? AND index_name = ?
             ORDER BY timestamp DESC
             LIMIT ?",
        )
        .bind::<Text, _>(market)
        .bind::<Text, _>(index_name)
        .bind::<Integer, _>(limit as i32)
        .load::<MarketSnapshotRow>(&mut conn)
        .map_err(StorageError::QueryFailed)?;
        rows.into_iter().map(TryInto::try_into).collect()
    }
}

#[async_trait]
impl CapitalFlowRepository for MarketIntelligenceSqliteRepository {
    async fn save_capital_flow_snapshots(&self, snapshots: &[CapitalFlowSnapshot]) -> Result<()> {
        let snapshots = snapshots.to_vec();
        self.writer
            .exec_tx(move |tx| {
                for snapshot in &snapshots {
                    sql_query(
                        "INSERT OR IGNORE INTO capital_flow_snapshots
                         (id, market, date, category, inflow, outflow, net_flow, source, created_at)
                         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    )
                    .bind::<Text, _>(stable_id(
                        "capital_flow",
                        &[
                            &snapshot.market,
                            &snapshot.date.to_string(),
                            &snapshot.category,
                            &snapshot.source,
                        ],
                    ))
                    .bind::<Text, _>(&snapshot.market)
                    .bind::<Text, _>(snapshot.date.to_string())
                    .bind::<Text, _>(&snapshot.category)
                    .bind::<Nullable<Double>, _>(snapshot.inflow)
                    .bind::<Nullable<Double>, _>(snapshot.outflow)
                    .bind::<Double, _>(snapshot.net_flow)
                    .bind::<Text, _>(&snapshot.source)
                    .bind::<Text, _>(Utc::now().to_rfc3339())
                    .execute(tx.conn())
                    .map_err(StorageError::QueryFailed)?;
                }
                Ok(())
            })
            .await
    }

    async fn capital_flow_snapshots(
        &self,
        market: Option<&str>,
        category: Option<&str>,
        since: Option<NaiveDate>,
        limit: usize,
    ) -> Result<Vec<CapitalFlowSnapshot>> {
        let mut conn = get_connection(&self.pool)?;
        let rows = sql_query(
            "SELECT market, date, category, inflow, outflow, net_flow, source
             FROM capital_flow_snapshots
             WHERE (? IS NULL OR market = ?)
               AND (? IS NULL OR category = ?)
               AND (? IS NULL OR date >= ?)
             ORDER BY date DESC
             LIMIT ?",
        )
        .bind::<Nullable<Text>, _>(market)
        .bind::<Nullable<Text>, _>(market)
        .bind::<Nullable<Text>, _>(category)
        .bind::<Nullable<Text>, _>(category)
        .bind::<Nullable<Text>, _>(since.map(|date| date.to_string()))
        .bind::<Nullable<Text>, _>(since.map(|date| date.to_string()))
        .bind::<Integer, _>(limit as i32)
        .load::<CapitalFlowRow>(&mut conn)
        .map_err(StorageError::QueryFailed)?;
        rows.into_iter().map(TryInto::try_into).collect()
    }
}

#[async_trait]
impl SectorRotationRepository for MarketIntelligenceSqliteRepository {
    async fn save_sector_rotation_snapshots(
        &self,
        snapshots: &[SectorRotationSnapshot],
    ) -> Result<()> {
        let snapshots = snapshots.to_vec();
        self.writer
            .exec_tx(move |tx| {
                for snapshot in &snapshots {
                    sql_query(
                        "INSERT OR IGNORE INTO sector_rotation_snapshots
                         (id, market, sector, date, net_flow, change_pct, turnover, ranking, source, created_at)
                         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    )
                    .bind::<Text, _>(stable_id(
                        "sector_rotation",
                        &[
                            &snapshot.market,
                            &snapshot.sector,
                            &snapshot.date.to_string(),
                            &snapshot.source,
                        ],
                    ))
                    .bind::<Text, _>(&snapshot.market)
                    .bind::<Text, _>(&snapshot.sector)
                    .bind::<Text, _>(snapshot.date.to_string())
                    .bind::<Nullable<Double>, _>(snapshot.net_flow)
                    .bind::<Nullable<Double>, _>(snapshot.change_pct)
                    .bind::<Nullable<Double>, _>(snapshot.turnover)
                    .bind::<Nullable<Integer>, _>(snapshot.ranking)
                    .bind::<Text, _>(&snapshot.source)
                    .bind::<Text, _>(Utc::now().to_rfc3339())
                    .execute(tx.conn())
                    .map_err(StorageError::QueryFailed)?;
                }
                Ok(())
            })
            .await
    }

    async fn sector_rotation_snapshots(
        &self,
        market: Option<&str>,
        sector: Option<&str>,
        since: Option<NaiveDate>,
        limit: usize,
    ) -> Result<Vec<SectorRotationSnapshot>> {
        let mut conn = get_connection(&self.pool)?;
        let rows = sql_query(
            "SELECT market, sector, date, net_flow, change_pct, turnover, ranking, source
             FROM sector_rotation_snapshots
             WHERE (? IS NULL OR market = ?)
               AND (? IS NULL OR sector = ?)
               AND (? IS NULL OR date >= ?)
             ORDER BY date DESC, ranking ASC
             LIMIT ?",
        )
        .bind::<Nullable<Text>, _>(market)
        .bind::<Nullable<Text>, _>(market)
        .bind::<Nullable<Text>, _>(sector)
        .bind::<Nullable<Text>, _>(sector)
        .bind::<Nullable<Text>, _>(since.map(|date| date.to_string()))
        .bind::<Nullable<Text>, _>(since.map(|date| date.to_string()))
        .bind::<Integer, _>(limit as i32)
        .load::<SectorRotationRow>(&mut conn)
        .map_err(StorageError::QueryFailed)?;
        rows.into_iter().map(TryInto::try_into).collect()
    }
}

#[async_trait]
impl ThemeRotationRepository for MarketIntelligenceSqliteRepository {
    async fn save_theme_rotation_snapshots(
        &self,
        snapshots: &[ThemeRotationSnapshot],
    ) -> Result<()> {
        let snapshots = snapshots.to_vec();
        self.writer
            .exec_tx(move |tx| {
                for snapshot in &snapshots {
                    sql_query(
                        "INSERT OR IGNORE INTO theme_rotation_snapshots
                         (id, theme, date, flow_score, momentum, ranking, created_at)
                         VALUES (?, ?, ?, ?, ?, ?, ?)",
                    )
                    .bind::<Text, _>(stable_id(
                        "theme_rotation",
                        &[&snapshot.theme, &snapshot.date.to_string()],
                    ))
                    .bind::<Text, _>(&snapshot.theme)
                    .bind::<Text, _>(snapshot.date.to_string())
                    .bind::<Nullable<Double>, _>(snapshot.flow_score)
                    .bind::<Nullable<Double>, _>(snapshot.momentum)
                    .bind::<Nullable<Integer>, _>(snapshot.ranking)
                    .bind::<Text, _>(Utc::now().to_rfc3339())
                    .execute(tx.conn())
                    .map_err(StorageError::QueryFailed)?;
                }
                Ok(())
            })
            .await
    }

    async fn theme_rotation_snapshots(
        &self,
        theme: Option<&str>,
        since: Option<NaiveDate>,
        limit: usize,
    ) -> Result<Vec<ThemeRotationSnapshot>> {
        let mut conn = get_connection(&self.pool)?;
        let rows = sql_query(
            "SELECT theme, date, flow_score, momentum, ranking
             FROM theme_rotation_snapshots
             WHERE (? IS NULL OR theme = ?)
               AND (? IS NULL OR date >= ?)
             ORDER BY date DESC, ranking ASC
             LIMIT ?",
        )
        .bind::<Nullable<Text>, _>(theme)
        .bind::<Nullable<Text>, _>(theme)
        .bind::<Nullable<Text>, _>(since.map(|date| date.to_string()))
        .bind::<Nullable<Text>, _>(since.map(|date| date.to_string()))
        .bind::<Integer, _>(limit as i32)
        .load::<ThemeRotationRow>(&mut conn)
        .map_err(StorageError::QueryFailed)?;
        rows.into_iter().map(TryInto::try_into).collect()
    }
}

#[async_trait]
impl PortfolioThemeExposureRepository for MarketIntelligenceSqliteRepository {
    async fn save_portfolio_theme_exposures(
        &self,
        exposures: &[PortfolioThemeExposure],
    ) -> Result<()> {
        let exposures = exposures.to_vec();
        self.writer
            .exec_tx(move |tx| {
                for exposure in &exposures {
                    sql_query(
                        "INSERT OR IGNORE INTO portfolio_theme_exposures
                         (id, portfolio_id, theme, weight_pct, market_value, source, timestamp)
                         VALUES (?, ?, ?, ?, ?, ?, ?)",
                    )
                    .bind::<Text, _>(stable_id(
                        "portfolio_theme_exposure",
                        &[
                            &exposure.portfolio_id,
                            &exposure.theme,
                            &exposure.timestamp.to_rfc3339(),
                            &exposure.source,
                        ],
                    ))
                    .bind::<Text, _>(&exposure.portfolio_id)
                    .bind::<Text, _>(&exposure.theme)
                    .bind::<Double, _>(exposure.weight_pct)
                    .bind::<Double, _>(exposure.market_value)
                    .bind::<Text, _>(&exposure.source)
                    .bind::<Text, _>(exposure.timestamp.to_rfc3339())
                    .execute(tx.conn())
                    .map_err(StorageError::QueryFailed)?;
                }
                Ok(())
            })
            .await
    }

    async fn latest_portfolio_theme_exposures(
        &self,
        portfolio_id: &str,
    ) -> Result<Vec<PortfolioThemeExposure>> {
        let mut conn = get_connection(&self.pool)?;
        let rows = sql_query(
            "SELECT portfolio_id, theme, weight_pct, market_value, source, timestamp
             FROM portfolio_theme_exposures
             WHERE portfolio_id = ?
             ORDER BY timestamp DESC",
        )
        .bind::<Text, _>(portfolio_id)
        .load::<PortfolioThemeExposureRow>(&mut conn)
        .map_err(StorageError::QueryFailed)?;
        latest_by_key(rows, |row| row.theme.clone())
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }
}

#[async_trait]
impl MarketIntelligenceIntradayRepository for MarketIntelligenceSqliteRepository {
    async fn save_intraday_snapshots(
        &self,
        snapshots: &[MarketIntelligenceIntradaySnapshot],
    ) -> Result<()> {
        let snapshots = snapshots.to_vec();
        self.writer
            .exec_tx(move |tx| {
                for snapshot in &snapshots {
                    sql_query(
                        "INSERT OR IGNORE INTO market_intelligence_intraday_snapshots
                         (id, market, kind, name, timestamp, open, close, high, low, volume,
                          amount, net_flow, change_pct, turnover, ranking, source, created_at)
                         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    )
                    .bind::<Text, _>(stable_id(
                        "market_intelligence_intraday",
                        &[
                            &snapshot.market,
                            &snapshot.kind,
                            &snapshot.name,
                            &snapshot.timestamp.to_rfc3339(),
                            &snapshot.source,
                        ],
                    ))
                    .bind::<Text, _>(&snapshot.market)
                    .bind::<Text, _>(&snapshot.kind)
                    .bind::<Text, _>(&snapshot.name)
                    .bind::<Text, _>(snapshot.timestamp.to_rfc3339())
                    .bind::<Nullable<Double>, _>(snapshot.open)
                    .bind::<Nullable<Double>, _>(snapshot.close)
                    .bind::<Nullable<Double>, _>(snapshot.high)
                    .bind::<Nullable<Double>, _>(snapshot.low)
                    .bind::<Nullable<Double>, _>(snapshot.volume)
                    .bind::<Nullable<Double>, _>(snapshot.amount)
                    .bind::<Nullable<Double>, _>(snapshot.net_flow)
                    .bind::<Nullable<Double>, _>(snapshot.change_pct)
                    .bind::<Nullable<Double>, _>(snapshot.turnover)
                    .bind::<Nullable<Integer>, _>(snapshot.ranking)
                    .bind::<Text, _>(&snapshot.source)
                    .bind::<Text, _>(Utc::now().to_rfc3339())
                    .execute(tx.conn())
                    .map_err(StorageError::QueryFailed)?;
                }
                Ok(())
            })
            .await
    }

    async fn intraday_snapshots(
        &self,
        kind: Option<&str>,
        name: Option<&str>,
        since: Option<DateTime<Utc>>,
        limit: usize,
    ) -> Result<Vec<MarketIntelligenceIntradaySnapshot>> {
        let mut conn = get_connection(&self.pool)?;
        let since = since.map(|value| value.to_rfc3339());
        let rows = sql_query(
            "SELECT market, kind, name, timestamp, open, close, high, low, volume,
                    amount, net_flow, change_pct, turnover, ranking, source
             FROM market_intelligence_intraday_snapshots
             WHERE (? IS NULL OR kind = ?)
               AND (? IS NULL OR name = ?)
               AND (? IS NULL OR timestamp >= ?)
             ORDER BY timestamp DESC, ranking ASC
             LIMIT ?",
        )
        .bind::<Nullable<Text>, _>(kind)
        .bind::<Nullable<Text>, _>(kind)
        .bind::<Nullable<Text>, _>(name)
        .bind::<Nullable<Text>, _>(name)
        .bind::<Nullable<Text>, _>(since.as_deref())
        .bind::<Nullable<Text>, _>(since.as_deref())
        .bind::<Integer, _>(limit as i32)
        .load::<IntradaySnapshotRow>(&mut conn)
        .map_err(StorageError::QueryFailed)?;
        rows.into_iter().map(TryInto::try_into).collect()
    }
}

#[async_trait]
impl MacroCapitalRepository for MarketIntelligenceSqliteRepository {
    async fn save_macro_capital_snapshots(&self, snapshots: &[MacroCapitalSnapshot]) -> Result<()> {
        let snapshots = snapshots.to_vec();
        self.writer
            .exec_tx(move |tx| {
                for snapshot in &snapshots {
                    sql_query(
                        "INSERT OR IGNORE INTO macro_capital_snapshots
                         (id, indicator, market, date, value, change, unit, source, created_at)
                         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    )
                    .bind::<Text, _>(stable_id(
                        "macro_capital",
                        &[
                            &snapshot.indicator,
                            &snapshot.market,
                            &snapshot.date.to_string(),
                            &snapshot.source,
                        ],
                    ))
                    .bind::<Text, _>(&snapshot.indicator)
                    .bind::<Text, _>(&snapshot.market)
                    .bind::<Text, _>(snapshot.date.to_string())
                    .bind::<Double, _>(snapshot.value)
                    .bind::<Nullable<Double>, _>(snapshot.change)
                    .bind::<Nullable<Text>, _>(snapshot.unit.clone())
                    .bind::<Text, _>(&snapshot.source)
                    .bind::<Text, _>(Utc::now().to_rfc3339())
                    .execute(tx.conn())
                    .map_err(StorageError::QueryFailed)?;
                }
                Ok(())
            })
            .await
    }

    async fn macro_capital_snapshots(
        &self,
        indicator: Option<&str>,
        market: Option<&str>,
        since: Option<NaiveDate>,
        limit: usize,
    ) -> Result<Vec<MacroCapitalSnapshot>> {
        let mut conn = get_connection(&self.pool)?;
        let rows = sql_query(
            "SELECT indicator, market, date, value, change, unit, source
             FROM macro_capital_snapshots
             WHERE (? IS NULL OR indicator = ?)
               AND (? IS NULL OR market = ?)
               AND (? IS NULL OR date >= ?)
             ORDER BY date DESC
             LIMIT ?",
        )
        .bind::<Nullable<Text>, _>(indicator)
        .bind::<Nullable<Text>, _>(indicator)
        .bind::<Nullable<Text>, _>(market)
        .bind::<Nullable<Text>, _>(market)
        .bind::<Nullable<Text>, _>(since.map(|date| date.to_string()))
        .bind::<Nullable<Text>, _>(since.map(|date| date.to_string()))
        .bind::<Integer, _>(limit as i32)
        .load::<MacroCapitalRow>(&mut conn)
        .map_err(StorageError::QueryFailed)?;
        rows.into_iter().map(TryInto::try_into).collect()
    }
}

fn latest_by_key<T, K>(rows: Vec<T>, key: impl Fn(&T) -> K) -> Vec<T>
where
    K: Ord,
{
    let mut seen = BTreeSet::new();
    rows.into_iter()
        .filter(|row| seen.insert(key(row)))
        .collect()
}

impl TryFrom<MarketSnapshotRow> for MarketSnapshot {
    type Error = Error;

    fn try_from(row: MarketSnapshotRow) -> Result<Self> {
        Ok(Self {
            market: row.market,
            index_name: row.index_name,
            price: row.price,
            change_pct: row.change_pct,
            turnover: row.turnover,
            timestamp: parse_datetime(&row.timestamp)?,
            source: row.source,
        })
    }
}

impl TryFrom<CapitalFlowRow> for CapitalFlowSnapshot {
    type Error = Error;

    fn try_from(row: CapitalFlowRow) -> Result<Self> {
        Ok(Self {
            market: row.market,
            date: parse_date(&row.date)?,
            category: row.category,
            inflow: row.inflow,
            outflow: row.outflow,
            net_flow: row.net_flow,
            source: row.source,
        })
    }
}

impl TryFrom<SectorRotationRow> for SectorRotationSnapshot {
    type Error = Error;

    fn try_from(row: SectorRotationRow) -> Result<Self> {
        Ok(Self {
            market: row.market,
            sector: row.sector,
            date: parse_date(&row.date)?,
            net_flow: row.net_flow,
            change_pct: row.change_pct,
            turnover: row.turnover,
            ranking: row.ranking,
            source: row.source,
        })
    }
}

impl TryFrom<ThemeRotationRow> for ThemeRotationSnapshot {
    type Error = Error;

    fn try_from(row: ThemeRotationRow) -> Result<Self> {
        Ok(Self {
            theme: row.theme,
            date: parse_date(&row.date)?,
            flow_score: row.flow_score,
            momentum: row.momentum,
            ranking: row.ranking,
        })
    }
}

impl TryFrom<PortfolioThemeExposureRow> for PortfolioThemeExposure {
    type Error = Error;

    fn try_from(row: PortfolioThemeExposureRow) -> Result<Self> {
        Ok(Self {
            portfolio_id: row.portfolio_id,
            theme: row.theme,
            weight_pct: row.weight_pct,
            market_value: row.market_value,
            source: row.source,
            timestamp: parse_datetime(&row.timestamp)?,
        })
    }
}

impl TryFrom<IntradaySnapshotRow> for MarketIntelligenceIntradaySnapshot {
    type Error = Error;

    fn try_from(row: IntradaySnapshotRow) -> Result<Self> {
        Ok(Self {
            market: row.market,
            kind: row.kind,
            name: row.name,
            timestamp: parse_datetime(&row.timestamp)?,
            open: row.open,
            close: row.close,
            high: row.high,
            low: row.low,
            volume: row.volume,
            amount: row.amount,
            net_flow: row.net_flow,
            change_pct: row.change_pct,
            turnover: row.turnover,
            ranking: row.ranking,
            source: row.source,
        })
    }
}

impl TryFrom<MacroCapitalRow> for MacroCapitalSnapshot {
    type Error = Error;

    fn try_from(row: MacroCapitalRow) -> Result<Self> {
        Ok(Self {
            indicator: row.indicator,
            market: row.market,
            date: parse_date(&row.date)?,
            value: row.value,
            change: row.change,
            unit: row.unit,
            source: row.source,
        })
    }
}

fn parse_date(value: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|error| Error::Repository(format!("Invalid market intelligence date: {error}")))
}

fn parse_datetime(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| {
            Error::Repository(format!("Invalid market intelligence timestamp: {error}"))
        })
}

fn stable_id(namespace: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(namespace.as_bytes());
    for part in parts {
        hasher.update(b":");
        hasher.update(part.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}
