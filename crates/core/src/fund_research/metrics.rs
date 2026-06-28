use super::models::{FundInternalHolding, FundNavPoint, HoldingConcentration};

pub fn momentum_pct(history: &[FundNavPoint], days: i64) -> Option<f64> {
    let latest = history.last()?;
    let cutoff = latest.date - chrono::Duration::days(days);
    let start = history
        .iter()
        .rev()
        .find(|point| point.date <= cutoff)
        .or_else(|| history.first())?;
    if start.nav <= 0.0 {
        return None;
    }
    Some((latest.nav / start.nav - 1.0) * 100.0)
}

pub fn max_drawdown_pct(history: &[FundNavPoint]) -> Option<f64> {
    let mut peak = None::<f64>;
    let mut max_drawdown = 0.0;
    for point in history {
        peak = Some(peak.map_or(point.nav, |value| value.max(point.nav)));
        let peak = peak?;
        if peak > 0.0 {
            let drawdown = (point.nav / peak - 1.0) * 100.0;
            if drawdown < max_drawdown {
                max_drawdown = drawdown;
            }
        }
    }
    Some(max_drawdown)
}

pub fn holding_concentration(holdings: &[FundInternalHolding]) -> HoldingConcentration {
    let mut weights: Vec<f64> = holdings
        .iter()
        .filter_map(|holding| holding.weight_pct)
        .filter(|weight| *weight > 0.0)
        .collect();
    weights.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

    let sum_top = |n: usize| -> Option<f64> {
        let sum: f64 = weights.iter().take(n).sum();
        (sum > 0.0).then_some(sum)
    };

    HoldingConcentration {
        top1_weight: weights.first().copied(),
        top5_weight: sum_top(5),
        top10_weight: sum_top(10),
        hhi: {
            let hhi: f64 = weights.iter().map(|weight| (weight / 100.0).powi(2)).sum();
            (hhi > 0.0).then_some(hhi)
        },
    }
}

pub fn weighted_average(values: &[(Option<f64>, f64)]) -> Option<f64> {
    let (weighted_sum, weight_sum) =
        values
            .iter()
            .fold((0.0, 0.0), |(weighted_sum, weight_sum), (value, weight)| {
                if let Some(value) = value {
                    (weighted_sum + value * weight, weight_sum + weight)
                } else {
                    (weighted_sum, weight_sum)
                }
            });
    (weight_sum > 0.0).then_some(weighted_sum / weight_sum)
}

pub fn weighted_score(parts: &[(Option<f64>, f64)]) -> Option<f64> {
    weighted_average(parts).map(|score| score.clamp(0.0, 100.0))
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    #[test]
    fn calculates_momentum() {
        let history = vec![
            FundNavPoint {
                date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                nav: 1.0,
                daily_return_pct: None,
            },
            FundNavPoint {
                date: NaiveDate::from_ymd_opt(2026, 2, 1).unwrap(),
                nav: 1.1,
                daily_return_pct: None,
            },
        ];
        assert!((momentum_pct(&history, 30).unwrap() - 10.0).abs() < 0.01);
    }

    #[test]
    fn weighted_score_renormalizes_missing_parts() {
        assert_eq!(
            weighted_score(&[(Some(80.0), 0.4), (None, 0.6)]),
            Some(80.0)
        );
    }
}
