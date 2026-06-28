use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sql_query;
use diesel::sql_types::{Double, Nullable, Text};
use diesel::sqlite::SqliteConnection;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::db::{get_connection, WriteHandle};
use crate::errors::StorageError;
use wealthfolio_core::errors::{Error, Result};
use wealthfolio_core::fund_research::{
    FundResearchRepository, FundResearchSnapshot, RebalanceAlert, SectorRotationSignal,
};

pub struct FundResearchSqliteRepository {
    pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
    writer: WriteHandle,
}

impl FundResearchSqliteRepository {
    pub fn new(pool: Arc<Pool<ConnectionManager<SqliteConnection>>>, writer: WriteHandle) -> Self {
        Self { pool, writer }
    }

    /// Strip HTML tags from legacy asset_name data in fund_internal_holdings,
    /// fund_holding_changes, AND the embedded snapshot_json column.
    /// Safe to call multiple times — idempotent.
    pub async fn cleanup_html_from_holdings(&self) -> Result<()> {
        let re = regex::Regex::new(r"<[^>]*>").unwrap();
        let mut conn = get_connection(&self.pool)?;

        // 1. Clean fund_internal_holdings.asset_name
        let rows = sql_query(
            "SELECT id, asset_name FROM fund_internal_holdings WHERE asset_name LIKE '%<%>%'",
        )
        .load::<NameRow>(&mut conn)
        .map_err(StorageError::QueryFailed)?;

        for row in rows {
            let cleaned = re.replace_all(&row.asset_name, "").trim().to_string();
            if cleaned != row.asset_name {
                sql_query("UPDATE fund_internal_holdings SET asset_name = ? WHERE id = ?")
                    .bind::<Text, _>(&cleaned)
                    .bind::<Text, _>(&row.id)
                    .execute(&mut conn)
                    .map_err(StorageError::QueryFailed)?;
            }
        }

        // 2. Clean fund_holding_changes.asset_name
        let rows = sql_query(
            "SELECT id, asset_name FROM fund_holding_changes WHERE asset_name LIKE '%<%>%'",
        )
        .load::<NameRow>(&mut conn)
        .map_err(StorageError::QueryFailed)?;

        for row in rows {
            let cleaned = re.replace_all(&row.asset_name, "").trim().to_string();
            if cleaned != row.asset_name {
                sql_query("UPDATE fund_holding_changes SET asset_name = ? WHERE id = ?")
                    .bind::<Text, _>(&cleaned)
                    .bind::<Text, _>(&row.id)
                    .execute(&mut conn)
                    .map_err(StorageError::QueryFailed)?;
            }
        }

        // 3. Clean snapshot_json column — this is where `get_fund_top_holdings` reads from.
        //    The snapshot_json embeds the full FundResearchSnapshot including fund_holdings.
        let rows = sql_query(
            "SELECT id, snapshot_json FROM fund_research_snapshots WHERE snapshot_json LIKE '%<%>%'",
        )
        .load::<SnapshotIdJsonRow>(&mut conn)
        .map_err(StorageError::QueryFailed)?;

        for row in rows {
            let cleaned = re.replace_all(&row.snapshot_json, "").to_string();
            if cleaned != row.snapshot_json {
                sql_query("UPDATE fund_research_snapshots SET snapshot_json = ? WHERE id = ?")
                    .bind::<Text, _>(&cleaned)
                    .bind::<Text, _>(&row.id)
                    .execute(&mut conn)
                    .map_err(StorageError::QueryFailed)?;
            }
        }

        log::info!("Cleaned HTML from fund research holdings (including snapshot_json)");
        Ok(())
    }
}

#[derive(QueryableByName)]
struct SnapshotJsonRow {
    #[diesel(sql_type = Text)]
    snapshot_json: String,
}

#[derive(QueryableByName)]
struct SnapshotIdJsonRow {
    #[diesel(sql_type = Text)]
    id: String,
    #[diesel(sql_type = Text)]
    snapshot_json: String,
}

#[derive(QueryableByName)]
struct NameRow {
    #[diesel(sql_type = Text)]
    id: String,
    #[diesel(sql_type = Text)]
    asset_name: String,
}

#[derive(QueryableByName)]
struct RotationSignalRow {
    #[diesel(sql_type = Text)]
    theme: String,
    #[diesel(sql_type = Text)]
    signal_date: String,
    #[diesel(sql_type = Nullable<Double>)]
    momentum_1m: Option<f64>,
    #[diesel(sql_type = Nullable<Double>)]
    momentum_3m: Option<f64>,
    #[diesel(sql_type = Nullable<Double>)]
    inflow_score: Option<f64>,
    #[diesel(sql_type = Nullable<Double>)]
    crowding_score: Option<f64>,
    #[diesel(sql_type = Nullable<Double>)]
    drawdown_risk: Option<f64>,
    #[diesel(sql_type = Text)]
    signal_strength: String,
    #[diesel(sql_type = Text)]
    reason_json: String,
}

#[derive(QueryableByName)]
struct RebalanceAlertRow {
    #[diesel(sql_type = Text)]
    id: String,
    #[diesel(sql_type = Text)]
    fund_code: String,
    #[diesel(sql_type = Text)]
    alert_date: String,
    #[diesel(sql_type = Text)]
    alert_type: String,
    #[diesel(sql_type = Text)]
    severity: String,
    #[diesel(sql_type = Text)]
    title: String,
    #[diesel(sql_type = Text)]
    message: String,
    #[diesel(sql_type = Text)]
    evidence_json: String,
    #[diesel(sql_type = Text)]
    created_at: String,
    #[diesel(sql_type = Nullable<Text>)]
    read_at: Option<String>,
}

#[async_trait]
impl FundResearchRepository for FundResearchSqliteRepository {
    async fn upsert_snapshot(&self, snapshot: &FundResearchSnapshot) -> Result<()> {
        let snapshot = snapshot.clone();
        self.writer
            .exec_tx(move |tx| {
                let conn = tx.conn();
                // Delete ALL old rows for this fund so stale snapshots never coexist.
                // REPLACE INTO alone isn't sufficient because the primary key
                // changes when snapshot_date differs between fetches.
                sql_query("DELETE FROM fund_research_snapshots WHERE fund_code = ?")
                    .bind::<Text, _>(&snapshot.fund_code)
                    .execute(conn)
                    .map_err(StorageError::QueryFailed)?;
                delete_snapshot_children(conn, &snapshot)?;
                insert_snapshot(conn, &snapshot)?;
                insert_snapshot_children(conn, &snapshot)?;
                Ok(())
            })
            .await
    }

    async fn latest_snapshot(&self, fund_code: &str) -> Result<Option<FundResearchSnapshot>> {
        let mut conn = get_connection(&self.pool)?;
        let row = sql_query(
            "SELECT snapshot_json FROM fund_research_snapshots
             WHERE fund_code = ?
             ORDER BY snapshot_date DESC, created_at DESC
             LIMIT 1",
        )
        .bind::<Text, _>(fund_code)
        .load::<SnapshotJsonRow>(&mut conn)
        .map_err(StorageError::QueryFailed)?
        .into_iter()
        .next();
        row.map(|row| deserialize_json(&row.snapshot_json))
            .transpose()
    }

    async fn snapshots_for_theme(
        &self,
        theme: &str,
        limit: usize,
    ) -> Result<Vec<FundResearchSnapshot>> {
        let mut conn = get_connection(&self.pool)?;
        let rows = sql_query(
            "SELECT DISTINCT s.snapshot_json
             FROM fund_research_snapshots s
             INNER JOIN fund_theme_exposures e
               ON e.fund_code = s.fund_code AND e.snapshot_date = s.snapshot_date
             WHERE e.theme = ?
             ORDER BY s.snapshot_date DESC
             LIMIT ?",
        )
        .bind::<Text, _>(theme)
        .bind::<diesel::sql_types::Integer, _>(limit as i32)
        .load::<SnapshotJsonRow>(&mut conn)
        .map_err(StorageError::QueryFailed)?;
        rows.into_iter()
            .map(|row| deserialize_json(&row.snapshot_json))
            .collect()
    }

    async fn latest_snapshots_for_funds(
        &self,
        fund_codes: &[String],
    ) -> Result<Vec<FundResearchSnapshot>> {
        let mut result = Vec::new();
        for fund_code in fund_codes {
            if let Some(snapshot) = self.latest_snapshot(fund_code).await? {
                result.push(snapshot);
            }
        }
        Ok(result)
    }

    async fn upsert_rotation_signals(&self, signals: &[SectorRotationSignal]) -> Result<()> {
        let signals = signals.to_vec();
        self.writer
            .exec_tx(move |tx| {
                for signal in &signals {
                    sql_query(
                        "REPLACE INTO sector_rotation_signals
                         (id, theme, signal_date, momentum_1m, momentum_3m, inflow_score,
                          crowding_score, drawdown_risk, signal_strength, reason_json, created_at)
                         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    )
                    .bind::<Text, _>(stable_id(
                        "rotation",
                        &[&signal.theme, &signal.signal_date.to_string()],
                    ))
                    .bind::<Text, _>(&signal.theme)
                    .bind::<Text, _>(signal.signal_date.to_string())
                    .bind::<Nullable<Double>, _>(signal.momentum_1m)
                    .bind::<Nullable<Double>, _>(signal.momentum_3m)
                    .bind::<Nullable<Double>, _>(signal.inflow_score)
                    .bind::<Nullable<Double>, _>(signal.crowding_score)
                    .bind::<Nullable<Double>, _>(signal.drawdown_risk)
                    .bind::<Text, _>(
                        serde_json::to_string(&signal.signal_strength).map_err(json_error)?,
                    )
                    .bind::<Text, _>(serde_json::to_string(&signal.reason).map_err(json_error)?)
                    .bind::<Text, _>(Utc::now().to_rfc3339())
                    .execute(tx.conn())
                    .map_err(StorageError::QueryFailed)?;
                }
                Ok(())
            })
            .await
    }

    async fn rotation_signals(&self, themes: &[String]) -> Result<Vec<SectorRotationSignal>> {
        let mut conn = get_connection(&self.pool)?;
        let mut signals = Vec::new();
        for theme in themes {
            let rows = sql_query(
                "SELECT theme, signal_date, momentum_1m, momentum_3m, inflow_score,
                        crowding_score, drawdown_risk, signal_strength, reason_json
                 FROM sector_rotation_signals
                 WHERE theme = ?
                 ORDER BY signal_date DESC",
            )
            .bind::<Text, _>(theme)
            .load::<RotationSignalRow>(&mut conn)
            .map_err(StorageError::QueryFailed)?;
            for row in rows {
                signals.push(row.try_into()?);
            }
        }
        Ok(signals)
    }

    async fn upsert_rebalance_alerts(&self, alerts: &[RebalanceAlert]) -> Result<()> {
        let alerts = alerts.to_vec();
        self.writer
            .exec_tx(move |tx| {
                for alert in &alerts {
                    sql_query(
                        "REPLACE INTO rebalance_alerts
                         (id, fund_code, alert_date, alert_type, severity, title, message,
                          evidence_json, created_at, read_at)
                         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    )
                    .bind::<Text, _>(&alert.id)
                    .bind::<Text, _>(&alert.fund_code)
                    .bind::<Text, _>(alert.alert_date.to_string())
                    .bind::<Text, _>(serde_json::to_string(&alert.alert_type).map_err(json_error)?)
                    .bind::<Text, _>(serde_json::to_string(&alert.severity).map_err(json_error)?)
                    .bind::<Text, _>(&alert.title)
                    .bind::<Text, _>(&alert.message)
                    .bind::<Text, _>(serde_json::to_string(&alert.evidence).map_err(json_error)?)
                    .bind::<Text, _>(alert.created_at.to_rfc3339())
                    .bind::<Nullable<Text>, _>(alert.read_at.map(|value| value.to_rfc3339()))
                    .execute(tx.conn())
                    .map_err(StorageError::QueryFailed)?;
                }
                Ok(())
            })
            .await
    }

    async fn rebalance_alerts(&self, fund_code: Option<&str>) -> Result<Vec<RebalanceAlert>> {
        let mut conn = get_connection(&self.pool)?;
        let rows = if let Some(fund_code) = fund_code {
            sql_query(
                "SELECT id, fund_code, alert_date, alert_type, severity, title, message,
                        evidence_json, created_at, read_at
                 FROM rebalance_alerts
                 WHERE fund_code = ?
                 ORDER BY read_at IS NOT NULL, alert_date DESC, created_at DESC",
            )
            .bind::<Text, _>(fund_code)
            .load::<RebalanceAlertRow>(&mut conn)
        } else {
            sql_query(
                "SELECT id, fund_code, alert_date, alert_type, severity, title, message,
                        evidence_json, created_at, read_at
                 FROM rebalance_alerts
                 ORDER BY read_at IS NOT NULL, alert_date DESC, created_at DESC",
            )
            .load::<RebalanceAlertRow>(&mut conn)
        }
        .map_err(StorageError::QueryFailed)?;

        rows.into_iter().map(TryInto::try_into).collect()
    }
}

fn insert_snapshot(conn: &mut SqliteConnection, snapshot: &FundResearchSnapshot) -> Result<()> {
    sql_query(
        "REPLACE INTO fund_research_snapshots
         (id, fund_code, fund_name, snapshot_date, source, quote_json, nav_history_json,
          performance_json, ai_features_json, snapshot_json, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(stable_id(
        "fund_snapshot",
        &[
            &snapshot.fund_code,
            &snapshot.snapshot_date.to_string(),
            &snapshot.source,
        ],
    ))
    .bind::<Text, _>(&snapshot.fund_code)
    .bind::<Text, _>(&snapshot.fund_name)
    .bind::<Text, _>(snapshot.snapshot_date.to_string())
    .bind::<Text, _>(&snapshot.source)
    .bind::<Nullable<Text>, _>(optional_json(&snapshot.quote)?)
    .bind::<Text, _>(to_json(&snapshot.nav_history)?)
    .bind::<Nullable<Text>, _>(optional_json(&snapshot.performance)?)
    .bind::<Text, _>(to_json(&snapshot.ai_features)?)
    .bind::<Text, _>(to_json(snapshot)?)
    .bind::<Text, _>(snapshot.created_at.to_rfc3339())
    .execute(conn)
    .map_err(StorageError::QueryFailed)?;
    Ok(())
}

fn delete_snapshot_children(
    conn: &mut SqliteConnection,
    snapshot: &FundResearchSnapshot,
) -> Result<()> {
    // Delete ALL child records for this fund regardless of date,
    // so stale data from older reports doesn't accumulate.
    for table in [
        "fund_internal_holdings",
        "fund_holding_changes",
        "fund_sector_allocations",
        "fund_theme_exposures",
        "fund_asset_allocations",
        "fund_region_allocations",
        "fund_risk_metrics",
        "fund_announcements",
    ] {
        let sql = format!("DELETE FROM {table} WHERE fund_code = ?");
        sql_query(sql)
            .bind::<Text, _>(&snapshot.fund_code)
            .execute(conn)
            .map_err(StorageError::QueryFailed)?;
    }
    Ok(())
}

fn insert_snapshot_children(
    conn: &mut SqliteConnection,
    snapshot: &FundResearchSnapshot,
) -> Result<()> {
    let created_at = snapshot.created_at.to_rfc3339();
    for holding in &snapshot.fund_holdings {
        sql_query(
            "REPLACE INTO fund_internal_holdings
             (id, fund_code, report_date, rank, asset_code, asset_name, asset_type, market,
              weight_pct, theme_tags_json, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind::<Text, _>(stable_id(
            "fund_holding",
            &[
                &holding.fund_code,
                &holding.report_date.to_string(),
                &holding.asset_name,
                holding.asset_code.as_deref().unwrap_or(""),
            ],
        ))
        .bind::<Text, _>(&holding.fund_code)
        .bind::<Text, _>(holding.report_date.to_string())
        .bind::<Nullable<diesel::sql_types::Integer>, _>(holding.rank.map(|rank| rank as i32))
        .bind::<Nullable<Text>, _>(holding.asset_code.clone())
        .bind::<Text, _>(&holding.asset_name)
        .bind::<Text, _>(to_json(&holding.asset_type)?)
        .bind::<Nullable<Text>, _>(holding.market.clone())
        .bind::<Nullable<Double>, _>(holding.weight_pct)
        .bind::<Text, _>(to_json(&holding.theme_tags)?)
        .bind::<Text, _>(&created_at)
        .execute(conn)
        .map_err(StorageError::QueryFailed)?;
    }

    for exposure in &snapshot.theme_exposures {
        sql_query(
            "REPLACE INTO fund_theme_exposures
             (id, fund_code, snapshot_date, theme, weight_pct, source, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind::<Text, _>(stable_id(
            "theme_exposure",
            &[
                &exposure.fund_code,
                &exposure.snapshot_date.to_string(),
                &exposure.theme,
                &to_json(&exposure.source)?,
            ],
        ))
        .bind::<Text, _>(&exposure.fund_code)
        .bind::<Text, _>(exposure.snapshot_date.to_string())
        .bind::<Text, _>(&exposure.theme)
        .bind::<Double, _>(exposure.weight_pct)
        .bind::<Text, _>(to_json(&exposure.source)?)
        .bind::<Text, _>(&created_at)
        .execute(conn)
        .map_err(StorageError::QueryFailed)?;
    }

    for change in &snapshot.holding_changes {
        sql_query(
            "INSERT INTO fund_holding_changes
             (id, fund_code, report_date, asset_code, asset_name, previous_weight_pct,
              current_weight_pct, weight_change_pct, previous_rank, current_rank, change_type, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind::<Text, _>(Uuid::new_v4().to_string())
        .bind::<Text, _>(&change.fund_code)
        .bind::<Text, _>(change.report_date.to_string())
        .bind::<Nullable<Text>, _>(change.asset_code.clone())
        .bind::<Text, _>(&change.asset_name)
        .bind::<Nullable<Double>, _>(change.previous_weight_pct)
        .bind::<Nullable<Double>, _>(change.current_weight_pct)
        .bind::<Nullable<Double>, _>(change.weight_change_pct)
        .bind::<Nullable<diesel::sql_types::Integer>, _>(change.previous_rank.map(|rank| rank as i32))
        .bind::<Nullable<diesel::sql_types::Integer>, _>(change.current_rank.map(|rank| rank as i32))
        .bind::<Text, _>(to_json(&change.change_type)?)
        .bind::<Text, _>(&created_at)
        .execute(conn)
        .map_err(StorageError::QueryFailed)?;
    }

    insert_json_children(conn, snapshot, &created_at)
}

fn insert_json_children(
    conn: &mut SqliteConnection,
    snapshot: &FundResearchSnapshot,
    created_at: &str,
) -> Result<()> {
    for sector in &snapshot.sector_allocations {
        sql_query(
            "REPLACE INTO fund_sector_allocations
             (id, fund_code, snapshot_date, sector, weight_pct, previous_weight_pct, weight_change_pct, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind::<Text, _>(stable_id("sector", &[&sector.fund_code, &sector.snapshot_date.to_string(), &sector.sector]))
        .bind::<Text, _>(&sector.fund_code)
        .bind::<Text, _>(sector.snapshot_date.to_string())
        .bind::<Text, _>(&sector.sector)
        .bind::<Nullable<Double>, _>(sector.weight_pct)
        .bind::<Nullable<Double>, _>(sector.previous_weight_pct)
        .bind::<Nullable<Double>, _>(sector.weight_change_pct)
        .bind::<Text, _>(created_at)
        .execute(conn)
        .map_err(StorageError::QueryFailed)?;
    }
    for asset in &snapshot.asset_allocations {
        sql_query(
            "REPLACE INTO fund_asset_allocations
             (id, fund_code, snapshot_date, asset_class, weight_pct, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind::<Text, _>(stable_id(
            "asset_alloc",
            &[
                &asset.fund_code,
                &asset.snapshot_date.to_string(),
                &asset.asset_class,
            ],
        ))
        .bind::<Text, _>(&asset.fund_code)
        .bind::<Text, _>(asset.snapshot_date.to_string())
        .bind::<Text, _>(&asset.asset_class)
        .bind::<Nullable<Double>, _>(asset.weight_pct)
        .bind::<Text, _>(created_at)
        .execute(conn)
        .map_err(StorageError::QueryFailed)?;
    }
    for region in &snapshot.region_allocations {
        sql_query(
            "REPLACE INTO fund_region_allocations
             (id, fund_code, snapshot_date, region, weight_pct, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind::<Text, _>(stable_id(
            "region_alloc",
            &[
                &region.fund_code,
                &region.snapshot_date.to_string(),
                &region.region,
            ],
        ))
        .bind::<Text, _>(&region.fund_code)
        .bind::<Text, _>(region.snapshot_date.to_string())
        .bind::<Text, _>(&region.region)
        .bind::<Nullable<Double>, _>(region.weight_pct)
        .bind::<Text, _>(created_at)
        .execute(conn)
        .map_err(StorageError::QueryFailed)?;
    }
    if let Some(risk) = &snapshot.risk_metrics {
        sql_query(
            "REPLACE INTO fund_risk_metrics
             (id, fund_code, snapshot_date, max_drawdown_1m_pct, max_drawdown_1y_pct,
              annualized_volatility_pct, sharpe_ratio, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind::<Text, _>(stable_id(
            "risk",
            &[&snapshot.fund_code, &snapshot.snapshot_date.to_string()],
        ))
        .bind::<Text, _>(&snapshot.fund_code)
        .bind::<Text, _>(snapshot.snapshot_date.to_string())
        .bind::<Nullable<Double>, _>(risk.max_drawdown_1m_pct)
        .bind::<Nullable<Double>, _>(risk.max_drawdown_1y_pct)
        .bind::<Nullable<Double>, _>(risk.annualized_volatility_pct)
        .bind::<Nullable<Double>, _>(risk.sharpe_ratio)
        .bind::<Text, _>(created_at)
        .execute(conn)
        .map_err(StorageError::QueryFailed)?;
    }
    for announcement in &snapshot.announcements {
        sql_query(
            "INSERT INTO fund_announcements
             (id, fund_code, announcement_date, title, url, category, importance, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind::<Text, _>(Uuid::new_v4().to_string())
        .bind::<Text, _>(&announcement.fund_code)
        .bind::<Nullable<Text>, _>(announcement.date.map(|date| date.to_string()))
        .bind::<Text, _>(&announcement.title)
        .bind::<Nullable<Text>, _>(announcement.url.clone())
        .bind::<Nullable<Text>, _>(optional_json(&announcement.category)?)
        .bind::<Text, _>(to_json(&announcement.importance)?)
        .bind::<Text, _>(created_at)
        .execute(conn)
        .map_err(StorageError::QueryFailed)?;
    }
    Ok(())
}

fn stable_id(prefix: &str, parts: &[&str]) -> String {
    let joined = parts.join("|");
    let digest = Sha256::digest(joined.as_bytes());
    let suffix = digest[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{prefix}:{suffix}")
}

fn to_json<T: serde::Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value).map_err(json_error)
}

fn optional_json<T: serde::Serialize>(value: &Option<T>) -> Result<Option<String>> {
    value.as_ref().map(to_json).transpose()
}

fn deserialize_json<T: serde::de::DeserializeOwned>(value: &str) -> Result<T> {
    serde_json::from_str(value).map_err(json_error)
}

fn json_error(error: serde_json::Error) -> Error {
    Error::Repository(format!("Fund research JSON error: {error}"))
}

impl TryFrom<RotationSignalRow> for SectorRotationSignal {
    type Error = Error;

    fn try_from(row: RotationSignalRow) -> Result<Self> {
        Ok(Self {
            theme: row.theme,
            signal_date: row.signal_date.parse().map_err(|error| {
                Error::Repository(format!("Invalid rotation signal date: {error}"))
            })?,
            momentum_1m: row.momentum_1m,
            momentum_3m: row.momentum_3m,
            inflow_score: row.inflow_score,
            crowding_score: row.crowding_score,
            drawdown_risk: row.drawdown_risk,
            signal_strength: deserialize_json(&row.signal_strength)?,
            reason: deserialize_json(&row.reason_json)?,
        })
    }
}

impl TryFrom<RebalanceAlertRow> for RebalanceAlert {
    type Error = Error;

    fn try_from(row: RebalanceAlertRow) -> Result<Self> {
        Ok(Self {
            id: row.id,
            fund_code: row.fund_code,
            alert_date: row.alert_date.parse().map_err(|error| {
                Error::Repository(format!("Invalid rebalance alert date: {error}"))
            })?,
            alert_type: deserialize_json(&row.alert_type)?,
            severity: deserialize_json(&row.severity)?,
            title: row.title,
            message: row.message,
            evidence: deserialize_json(&row.evidence_json)?,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|error| Error::Repository(format!("Invalid alert created_at: {error}")))?
                .with_timezone(&Utc),
            read_at: row
                .read_at
                .map(|value| {
                    chrono::DateTime::parse_from_rfc3339(&value)
                        .map(|date| date.with_timezone(&Utc))
                        .map_err(|error| {
                            Error::Repository(format!("Invalid alert read_at: {error}"))
                        })
                })
                .transpose()?,
        })
    }
}
