use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::market_intelligence::sector_rotation::SectorRotationSnapshot;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThemeMapping {
    pub theme: String,
    pub sectors: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThemeRotationSnapshot {
    pub theme: String,
    pub date: NaiveDate,
    pub flow_score: Option<f64>,
    pub momentum: Option<f64>,
    pub ranking: Option<i32>,
}

#[derive(Clone, Debug, Default)]
pub struct ThemeRotationRules {
    mappings: Vec<ThemeMapping>,
}

impl ThemeRotationRules {
    pub fn new(mappings: Vec<ThemeMapping>) -> Self {
        Self { mappings }
    }

    pub fn calculate(
        &self,
        sector_snapshots: &[SectorRotationSnapshot],
    ) -> Vec<ThemeRotationSnapshot> {
        let mut snapshots: Vec<ThemeRotationSnapshot> = self
            .mappings
            .iter()
            .filter_map(|mapping| calculate_theme_snapshot(mapping, sector_snapshots))
            .collect();
        snapshots.sort_by(|a, b| {
            theme_rank_value(b)
                .partial_cmp(&theme_rank_value(a))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for (index, snapshot) in snapshots.iter_mut().enumerate() {
            snapshot.ranking = Some((index + 1) as i32);
        }
        snapshots
    }
}

fn calculate_theme_snapshot(
    mapping: &ThemeMapping,
    sector_snapshots: &[SectorRotationSnapshot],
) -> Option<ThemeRotationSnapshot> {
    let matched: Vec<&SectorRotationSnapshot> = sector_snapshots
        .iter()
        .filter(|snapshot| {
            mapping
                .sectors
                .iter()
                .any(|sector| sector.eq_ignore_ascii_case(&snapshot.sector))
        })
        .collect();
    let first = matched.first()?;
    let flow_score = average(matched.iter().filter_map(|snapshot| snapshot.net_flow))
        .or_else(|| average(matched.iter().filter_map(|snapshot| snapshot.change_pct)));
    let momentum = average(matched.iter().filter_map(|snapshot| snapshot.change_pct));
    Some(ThemeRotationSnapshot {
        theme: mapping.theme.clone(),
        date: first.date,
        flow_score,
        momentum,
        ranking: None,
    })
}

fn theme_rank_value(snapshot: &ThemeRotationSnapshot) -> f64 {
    snapshot
        .flow_score
        .or(snapshot.momentum)
        .unwrap_or(f64::NEG_INFINITY)
}

fn average(values: impl Iterator<Item = f64>) -> Option<f64> {
    let (sum, count) = values.fold((0.0, 0_usize), |(sum, count), value| {
        (sum + value, count + 1)
    });
    (count > 0).then_some(sum / count as f64)
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    #[test]
    fn calculates_theme_rotation_from_sector_rules() {
        let date = NaiveDate::from_ymd_opt(2026, 6, 30).unwrap();
        let rules = ThemeRotationRules::new(vec![ThemeMapping {
            theme: "AI".to_string(),
            sectors: vec!["GPU".to_string(), "CPO".to_string()],
        }]);
        let snapshots = vec![
            sector("GPU", date, 10.0, 3.0),
            sector("CPO", date, 6.0, 1.0),
        ];

        let result = rules.calculate(&snapshots);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].theme, "AI");
        assert_eq!(result[0].flow_score, Some(8.0));
        assert_eq!(result[0].momentum, Some(2.0));
        assert_eq!(result[0].ranking, Some(1));
    }

    fn sector(
        sector: &str,
        date: NaiveDate,
        net_flow: f64,
        change_pct: f64,
    ) -> SectorRotationSnapshot {
        SectorRotationSnapshot {
            market: "CN".to_string(),
            sector: sector.to_string(),
            date,
            net_flow: Some(net_flow),
            change_pct: Some(change_pct),
            turnover: None,
            ranking: None,
            source: "test".to_string(),
        }
    }
}
