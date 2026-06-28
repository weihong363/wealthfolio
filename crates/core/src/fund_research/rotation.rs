use std::collections::BTreeMap;

use chrono::NaiveDate;

use super::metrics::{momentum_pct, weighted_average, weighted_score};
use super::models::{
    FundResearchSnapshot, RotationSignalStrength, SectorRotationSignal, ThemeExposure,
};

#[derive(Clone, Debug, Default)]
pub struct RotationScoreInputs {
    pub scale_growth_score: Option<f64>,
    pub share_growth_score: Option<f64>,
    pub theme_exposure_change: Option<f64>,
    pub performance_rank_score: Option<f64>,
    pub top10_concentration: Option<f64>,
    pub theme_exposure: Option<f64>,
    pub recent_return_acceleration: Option<f64>,
}

pub fn inflow_score(inputs: &RotationScoreInputs) -> Option<f64> {
    weighted_score(&[
        (inputs.scale_growth_score, 0.4),
        (inputs.share_growth_score, 0.3),
        (inputs.theme_exposure_change, 0.2),
        (inputs.performance_rank_score, 0.1),
    ])
}

pub fn crowding_score(inputs: &RotationScoreInputs) -> Option<f64> {
    weighted_score(&[
        (inputs.top10_concentration, 0.4),
        (inputs.theme_exposure, 0.3),
        (inputs.recent_return_acceleration, 0.2),
        (inputs.scale_growth_score, 0.1),
    ])
}

pub fn classify_rotation_signal(
    momentum_1m: Option<f64>,
    momentum_3m: Option<f64>,
    inflow_score: Option<f64>,
    drawdown_risk: Option<f64>,
    exposure_delta: Option<f64>,
    has_negative_announcement: bool,
) -> (RotationSignalStrength, Vec<String>) {
    let mut reason = Vec::new();
    let m1 = momentum_1m.unwrap_or_default();
    let m3 = momentum_3m.unwrap_or_default();
    let inflow = inflow_score.unwrap_or_default();
    let drawdown_ok = drawdown_risk.map(|risk| risk > -25.0).unwrap_or(true);

    if m1 > 5.0 && m3 > 10.0 && inflow > 70.0 && drawdown_ok {
        reason.push("1m and 3m momentum are strong with high inflow proxy".to_string());
        return (RotationSignalStrength::StrongInflow, reason);
    }
    if m1 > 2.0 && m3 > 5.0 {
        reason.push("1m and 3m momentum are positive".to_string());
        return (RotationSignalStrength::Inflow, reason);
    }
    if m1 < -5.0 && m3 < -10.0 && (inflow < 40.0 || has_negative_announcement) {
        reason.push("momentum is sharply negative with weak flow or negative news".to_string());
        return (RotationSignalStrength::StrongOutflow, reason);
    }
    if m1 < -2.0 && exposure_delta.unwrap_or_default() < 0.0 {
        reason.push("1m momentum and theme exposure are falling".to_string());
        return (RotationSignalStrength::Outflow, reason);
    }

    reason.push("signal is mixed or missing required inputs".to_string());
    (RotationSignalStrength::Neutral, reason)
}

pub fn analyze_theme_rotation_from_snapshots(
    themes: &[String],
    snapshots: &[FundResearchSnapshot],
) -> Vec<SectorRotationSignal> {
    let latest_date = snapshots
        .iter()
        .map(|snapshot| snapshot.snapshot_date)
        .max()
        .unwrap_or_else(|| chrono::Utc::now().date_naive());

    themes
        .iter()
        .map(|theme| analyze_one_theme(theme, latest_date, snapshots))
        .collect()
}

fn analyze_one_theme(
    theme: &str,
    signal_date: NaiveDate,
    snapshots: &[FundResearchSnapshot],
) -> SectorRotationSignal {
    let mut weighted_momentum_1m = Vec::new();
    let mut weighted_momentum_3m = Vec::new();
    let mut weighted_inflow = Vec::new();
    let mut weighted_crowding = Vec::new();
    let mut weighted_drawdown = Vec::new();
    let mut exposure_delta = Vec::new();
    let mut negative_announcement = false;

    let previous_by_fund = previous_theme_exposure_by_fund(theme, snapshots);

    for snapshot in snapshots {
        let Some(exposure) = exposure_for_theme(&snapshot.theme_exposures, theme) else {
            continue;
        };
        let weight = exposure.weight_pct.max(0.0);
        if weight <= 0.0 {
            continue;
        }

        weighted_momentum_1m.push((momentum_pct(&snapshot.nav_history, 30), weight));
        weighted_momentum_3m.push((momentum_pct(&snapshot.nav_history, 90), weight));
        weighted_drawdown.push((
            snapshot
                .risk_metrics
                .as_ref()
                .and_then(|risk| risk.max_drawdown_1y_pct),
            weight,
        ));

        let previous = previous_by_fund.get(&snapshot.fund_code).copied();
        let delta = previous.map(|previous| exposure.weight_pct - previous);
        exposure_delta.push((delta, weight));

        let concentration = snapshot
            .ai_features
            .holding_concentration
            .as_ref()
            .and_then(|value| value.top10_weight);
        let inputs = RotationScoreInputs {
            theme_exposure: Some(exposure.weight_pct),
            theme_exposure_change: delta.map(|value| (value + 50.0).clamp(0.0, 100.0)),
            top10_concentration: concentration,
            recent_return_acceleration: momentum_pct(&snapshot.nav_history, 30)
                .zip(momentum_pct(&snapshot.nav_history, 90))
                .map(|(m1, m3)| (m1 - m3 / 3.0 + 50.0).clamp(0.0, 100.0)),
            ..Default::default()
        };
        weighted_inflow.push((inflow_score(&inputs), weight));
        weighted_crowding.push((crowding_score(&inputs), weight));

        negative_announcement |= snapshot.announcements.iter().any(|announcement| {
            matches!(
                announcement.category,
                Some(super::models::AnnouncementCategory::RiskWarning)
                    | Some(super::models::AnnouncementCategory::MajorEvent)
            )
        });
    }

    let momentum_1m = weighted_average(&weighted_momentum_1m);
    let momentum_3m = weighted_average(&weighted_momentum_3m);
    let inflow = weighted_average(&weighted_inflow);
    let crowding = weighted_average(&weighted_crowding);
    let drawdown = weighted_average(&weighted_drawdown);
    let exposure_delta = weighted_average(&exposure_delta);
    let (signal_strength, reason) = classify_rotation_signal(
        momentum_1m,
        momentum_3m,
        inflow,
        drawdown,
        exposure_delta,
        negative_announcement,
    );

    SectorRotationSignal {
        theme: theme.to_string(),
        signal_date,
        momentum_1m,
        momentum_3m,
        inflow_score: inflow,
        crowding_score: crowding,
        drawdown_risk: drawdown,
        signal_strength,
        reason,
    }
}

fn exposure_for_theme<'a>(
    exposures: &'a [ThemeExposure],
    theme: &str,
) -> Option<&'a ThemeExposure> {
    exposures
        .iter()
        .find(|exposure| exposure.theme.eq_ignore_ascii_case(theme))
}

fn previous_theme_exposure_by_fund(
    theme: &str,
    snapshots: &[FundResearchSnapshot],
) -> BTreeMap<String, f64> {
    let mut by_fund: BTreeMap<String, Vec<&FundResearchSnapshot>> = BTreeMap::new();
    for snapshot in snapshots {
        by_fund
            .entry(snapshot.fund_code.clone())
            .or_default()
            .push(snapshot);
    }

    by_fund
        .into_iter()
        .filter_map(|(fund_code, mut snapshots)| {
            snapshots.sort_by_key(|snapshot| snapshot.snapshot_date);
            snapshots
                .iter()
                .rev()
                .nth(1)
                .and_then(|snapshot| exposure_for_theme(&snapshot.theme_exposures, theme))
                .map(|exposure| (fund_code, exposure.weight_pct))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inflow_score_skips_missing_inputs() {
        let score = inflow_score(&RotationScoreInputs {
            scale_growth_score: Some(80.0),
            share_growth_score: None,
            theme_exposure_change: Some(60.0),
            performance_rank_score: None,
            ..Default::default()
        });
        assert!((score.unwrap() - 73.33333333333334).abs() < 0.000_001);
    }

    #[test]
    fn generates_strong_inflow_signal() {
        let (signal, reason) =
            classify_rotation_signal(Some(6.0), Some(12.0), Some(80.0), Some(-10.0), None, false);
        assert_eq!(signal, RotationSignalStrength::StrongInflow);
        assert!(!reason.is_empty());
    }
}
