use chrono::{NaiveDate, Utc};
use uuid::Uuid;

use super::models::{
    AlertSeverity, AnnouncementCategory, AnnouncementImportance, FundResearchSnapshot,
    RebalanceAlert, RebalanceAlertType,
};

pub fn generate_rebalance_alerts(
    current: &FundResearchSnapshot,
    previous: Option<&FundResearchSnapshot>,
) -> Vec<RebalanceAlert> {
    let mut alerts = Vec::new();
    alerts.extend(manager_and_announcement_alerts(current));
    alerts.extend(risk_alerts(current));
    if let Some(previous) = previous {
        alerts.extend(holding_exit_alerts(current, previous));
        alerts.extend(theme_drop_alerts(current, previous));
    }
    alerts
}

fn manager_and_announcement_alerts(snapshot: &FundResearchSnapshot) -> Vec<RebalanceAlert> {
    snapshot
        .announcements
        .iter()
        .filter_map(|announcement| match announcement.category {
            Some(AnnouncementCategory::ManagerChange) => Some(alert(
                &snapshot.fund_code,
                snapshot.snapshot_date,
                RebalanceAlertType::ManagerChanged,
                AlertSeverity::High,
                "Fund manager changed",
                &announcement.title,
                vec![announcement.title.clone()],
            )),
            Some(AnnouncementCategory::PurchaseSuspension)
            | Some(AnnouncementCategory::RedemptionSuspension) => Some(alert(
                &snapshot.fund_code,
                snapshot.snapshot_date,
                RebalanceAlertType::TradingSuspension,
                AlertSeverity::High,
                "Fund trading status changed",
                &announcement.title,
                vec![announcement.title.clone()],
            )),
            Some(AnnouncementCategory::MajorEvent) => Some(alert(
                &snapshot.fund_code,
                snapshot.snapshot_date,
                RebalanceAlertType::MajorAnnouncement,
                if announcement.importance == AnnouncementImportance::High {
                    AlertSeverity::Critical
                } else {
                    AlertSeverity::Medium
                },
                "Major fund announcement",
                &announcement.title,
                vec![announcement.title.clone()],
            )),
            _ => None,
        })
        .collect()
}

fn risk_alerts(snapshot: &FundResearchSnapshot) -> Vec<RebalanceAlert> {
    let mut alerts = Vec::new();
    let Some(risk) = snapshot.risk_metrics.as_ref() else {
        return alerts;
    };

    if risk.max_drawdown_1m_pct.is_some_and(|value| value <= -10.0) {
        alerts.push(alert(
            &snapshot.fund_code,
            snapshot.snapshot_date,
            RebalanceAlertType::DrawdownExceeded,
            AlertSeverity::High,
            "1-month drawdown exceeded threshold",
            "Recent drawdown is larger than the configured first-version threshold.",
            vec![format!(
                "max_drawdown_1m_pct={:.2}",
                risk.max_drawdown_1m_pct.unwrap_or_default()
            )],
        ));
    }
    if risk.sharpe_ratio.is_some_and(|value| value < 0.2) {
        alerts.push(alert(
            &snapshot.fund_code,
            snapshot.snapshot_date,
            RebalanceAlertType::SharpeDeteriorated,
            AlertSeverity::Medium,
            "Sharpe ratio deteriorated",
            "Risk-adjusted return has weakened.",
            vec![format!(
                "sharpe_ratio={:.2}",
                risk.sharpe_ratio.unwrap_or_default()
            )],
        ));
    }
    if risk
        .annualized_volatility_pct
        .is_some_and(|value| value > 35.0)
    {
        alerts.push(alert(
            &snapshot.fund_code,
            snapshot.snapshot_date,
            RebalanceAlertType::VolatilityIncreased,
            AlertSeverity::Medium,
            "Volatility increased",
            "Annualized volatility is above the first-version threshold.",
            vec![format!(
                "annualized_volatility_pct={:.2}",
                risk.annualized_volatility_pct.unwrap_or_default()
            )],
        ));
    }

    alerts
}

fn holding_exit_alerts(
    current: &FundResearchSnapshot,
    previous: &FundResearchSnapshot,
) -> Vec<RebalanceAlert> {
    previous
        .fund_holdings
        .iter()
        .filter(|holding| holding.rank.is_some_and(|rank| rank <= 3))
        .filter(|holding| {
            !current
                .fund_holdings
                .iter()
                .any(|next| next.asset_name == holding.asset_name)
        })
        .map(|holding| {
            alert(
                &current.fund_code,
                current.snapshot_date,
                RebalanceAlertType::CoreHoldingRemoved,
                AlertSeverity::High,
                "Core holding exited top holdings",
                &format!("{} is no longer in reported holdings.", holding.asset_name),
                vec![format!(
                    "previous_rank={} asset={}",
                    holding.rank.unwrap_or_default(),
                    holding.asset_name
                )],
            )
        })
        .collect()
}

fn theme_drop_alerts(
    current: &FundResearchSnapshot,
    previous: &FundResearchSnapshot,
) -> Vec<RebalanceAlert> {
    previous
        .theme_exposures
        .iter()
        .filter_map(|previous_exposure| {
            let current_weight = current
                .theme_exposures
                .iter()
                .find(|exposure| exposure.theme == previous_exposure.theme)
                .map(|exposure| exposure.weight_pct)
                .unwrap_or_default();
            let drop = previous_exposure.weight_pct - current_weight;
            (drop > 10.0).then(|| {
                alert(
                    &current.fund_code,
                    current.snapshot_date,
                    RebalanceAlertType::ThemeExposureDrop,
                    AlertSeverity::Medium,
                    "Theme exposure dropped",
                    &format!(
                        "{} exposure dropped by {:.2} percentage points.",
                        previous_exposure.theme, drop
                    ),
                    vec![format!(
                        "{}: {:.2}% -> {:.2}%",
                        previous_exposure.theme, previous_exposure.weight_pct, current_weight
                    )],
                )
            })
        })
        .collect()
}

fn alert(
    fund_code: &str,
    alert_date: NaiveDate,
    alert_type: RebalanceAlertType,
    severity: AlertSeverity,
    title: &str,
    message: &str,
    evidence: Vec<String>,
) -> RebalanceAlert {
    RebalanceAlert {
        id: Uuid::new_v4().to_string(),
        fund_code: fund_code.to_string(),
        alert_date,
        alert_type,
        severity,
        title: title.to_string(),
        message: message.to_string(),
        evidence,
        created_at: Utc::now(),
        read_at: None,
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::fund_research::models::{
        FundAiFeatures, FundRiskMetrics, ThemeExposure, ThemeExposureSource,
    };

    fn snapshot(weight: f64) -> FundResearchSnapshot {
        FundResearchSnapshot {
            fund_code: "014002".to_string(),
            fund_name: "Test".to_string(),
            snapshot_date: NaiveDate::from_ymd_opt(2026, 6, 27).unwrap(),
            source: "test".to_string(),
            quote: None,
            nav_history: vec![],
            fund_holdings: vec![],
            holding_changes: vec![],
            sector_allocations: vec![],
            theme_exposures: vec![ThemeExposure {
                fund_code: "014002".to_string(),
                snapshot_date: NaiveDate::from_ymd_opt(2026, 6, 27).unwrap(),
                theme: "AI".to_string(),
                weight_pct: weight,
                source: ThemeExposureSource::HoldingMapping,
            }],
            asset_allocations: vec![],
            region_allocations: vec![],
            performance: None,
            risk_metrics: Some(FundRiskMetrics {
                max_drawdown_1m_pct: Some(-11.0),
                ..Default::default()
            }),
            announcements: vec![],
            ai_features: FundAiFeatures::default(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn generates_drawdown_and_theme_drop_alerts() {
        let previous = snapshot(50.0);
        let current = snapshot(35.0);
        let alerts = generate_rebalance_alerts(&current, Some(&previous));
        assert!(alerts
            .iter()
            .any(|alert| alert.alert_type == RebalanceAlertType::ThemeExposureDrop));
        assert!(alerts
            .iter()
            .any(|alert| alert.alert_type == RebalanceAlertType::DrawdownExceeded));
    }
}
