use std::collections::BTreeMap;

use super::models::{
    FundLookthroughContribution, FundResearchSnapshot, FundTopHolding,
    PortfolioFundLookthroughHolding, PortfolioFundLookthroughSummary, PortfolioFundPosition,
    PortfolioThemeExposure, ThemeExposure,
};

// ─────────────────────────────────────────────────────────────
// Theme Lookthrough (existing)
// ─────────────────────────────────────────────────────────────

pub fn lookthrough_theme_exposure(
    portfolio_id: &str,
    positions: &[PortfolioFundPosition],
    snapshots: &[FundResearchSnapshot],
) -> Vec<PortfolioThemeExposure> {
    let total_value: f64 = positions
        .iter()
        .map(|position| position.market_value_base.max(0.0))
        .sum();
    if total_value <= 0.0 {
        return Vec::new();
    }

    let mut by_theme: BTreeMap<String, (f64, Vec<String>)> = BTreeMap::new();
    for position in positions {
        let Some(snapshot) = snapshots
            .iter()
            .find(|snapshot| snapshot.fund_code == position.fund_code)
        else {
            continue;
        };
        for exposure in &snapshot.theme_exposures {
            add_exposure(&mut by_theme, position, exposure);
        }
    }

    by_theme
        .into_iter()
        .map(|(theme, (value, source_funds))| PortfolioThemeExposure {
            portfolio_id: portfolio_id.to_string(),
            theme,
            exposure_value_base: value,
            weight_pct: value / total_value * 100.0,
            source_funds,
        })
        .collect()
}

fn add_exposure(
    by_theme: &mut BTreeMap<String, (f64, Vec<String>)>,
    position: &PortfolioFundPosition,
    exposure: &ThemeExposure,
) {
    let exposure_value = position.market_value_base * exposure.weight_pct / 100.0;
    let entry = by_theme
        .entry(exposure.theme.clone())
        .or_insert_with(|| (0.0, Vec::new()));
    entry.0 += exposure_value;
    if !entry.1.contains(&position.fund_code) {
        entry.1.push(position.fund_code.clone());
    }
}

// ─────────────────────────────────────────────────────────────
// Fund Holdings Lookthrough
// ─────────────────────────────────────────────────────────────

/// Extract top holdings (rank <= 10) from a fund snapshot.
/// Filters out holdings with missing weight_pct.
pub fn fund_top_holdings(snapshot: &FundResearchSnapshot) -> Vec<FundTopHolding> {
    let mut holdings: Vec<&_> = snapshot
        .fund_holdings
        .iter()
        .filter(|h| h.weight_pct.is_some())
        .collect();

    // Sort by rank ascending, putting None ranks at the end
    holdings.sort_by(|a, b| match (a.rank, b.rank) {
        (Some(ra), Some(rb)) => ra.cmp(&rb),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });

    holdings
        .into_iter()
        .take(10)
        .map(|h| FundTopHolding {
            fund_code: h.fund_code.clone(),
            fund_name: Some(snapshot.fund_name.clone()),
            report_date: h.report_date,
            rank: h.rank,
            asset_code: h.asset_code.clone(),
            asset_name: h.asset_name.clone(),
            asset_type: h.asset_type.clone(),
            market: h.market.clone(),
            weight_pct: h.weight_pct.unwrap_or(0.0),
            theme_tags: h.theme_tags.clone(),
        })
        .collect()
}

/// Aggregate fund internal holdings across all user positions.
///
/// * `portfolio_id` — identifier for the portfolio being analyzed.
/// * `positions` — user's fund positions (fund_code → market_value).
/// * `snapshots` — latest fund research snapshots for lookthrough.
pub fn portfolio_fund_lookthrough(
    portfolio_id: &str,
    positions: &[PortfolioFundPosition],
    snapshots: &[FundResearchSnapshot],
) -> PortfolioFundLookthroughSummary {
    let total_value: f64 = positions.iter().map(|p| p.market_value_base.max(0.0)).sum();

    let mut missing_funds: Vec<String> = Vec::new();

    // Group: holdings_key → (total_exposure, contributions, theme_tags)
    let mut by_asset: BTreeMap<HoldingsKey, (f64, Vec<FundLookthroughContribution>, Vec<String>)> =
        BTreeMap::new();

    for position in positions {
        let Some(snapshot) = snapshots.iter().find(|s| s.fund_code == position.fund_code) else {
            if !missing_funds.contains(&position.fund_code) {
                missing_funds.push(position.fund_code.clone());
            }
            continue;
        };

        for holding in &snapshot.fund_holdings {
            let weight = match holding.weight_pct {
                Some(w) if w > 0.0 => w,
                _ => continue,
            };
            let exposure_value = position.market_value_base * weight / 100.0;

            let key = HoldingsKey {
                asset_code: holding.asset_code.clone(),
                asset_name: holding.asset_name.clone(),
                asset_type: holding.asset_type.clone(),
                market: holding.market.clone(),
            };

            let entry = by_asset
                .entry(key.clone())
                .or_insert_with(|| (0.0, Vec::new(), Vec::new()));
            entry.0 += exposure_value;
            entry.1.push(FundLookthroughContribution {
                fund_code: position.fund_code.clone(),
                fund_name: Some(snapshot.fund_name.clone()),
                fund_market_value_base: position.market_value_base,
                fund_holding_weight_pct: weight,
                exposure_value_base: exposure_value,
                report_date: holding.report_date,
            });
            for tag in &holding.theme_tags {
                if !entry.2.contains(tag) {
                    entry.2.push(tag.clone());
                }
            }
        }
    }

    let mut holdings: Vec<PortfolioFundLookthroughHolding> = by_asset
        .into_iter()
        .map(
            |(key, (total_exposure, source_funds, theme_tags))| PortfolioFundLookthroughHolding {
                asset_code: key.asset_code,
                asset_name: key.asset_name,
                asset_type: key.asset_type,
                market: key.market,
                theme_tags,
                exposure_value_base: total_exposure,
                weight_pct: if total_value > 0.0 {
                    total_exposure / total_value * 100.0
                } else {
                    0.0
                },
                source_funds,
            },
        )
        .collect();

    // Sort by exposure value descending
    holdings.sort_by(|a, b| {
        b.exposure_value_base
            .partial_cmp(&a.exposure_value_base)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    PortfolioFundLookthroughSummary {
        portfolio_id: portfolio_id.to_string(),
        total_fund_market_value_base: total_value,
        holdings,
        missing_funds,
    }
}

/// Composite key for lookthrough aggregation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct HoldingsKey {
    asset_code: Option<String>,
    asset_name: String,
    asset_type: super::models::HoldingAssetType,
    market: Option<String>,
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, Utc};

    use super::*;
    use crate::fund_research::models::{FundAiFeatures, FundInternalHolding, HoldingAssetType, ThemeExposureSource};

    #[test]
    fn calculates_user_theme_lookthrough() {
        let date = NaiveDate::from_ymd_opt(2026, 6, 27).unwrap();
        let positions = vec![PortfolioFundPosition {
            portfolio_id: "p1".to_string(),
            fund_code: "014002".to_string(),
            market_value_base: 100_000.0,
        }];
        let snapshots = vec![FundResearchSnapshot {
            fund_code: "014002".to_string(),
            fund_name: "Test".to_string(),
            snapshot_date: date,
            source: "test".to_string(),
            quote: None,
            nav_history: vec![],
            fund_holdings: vec![],
            holding_changes: vec![],
            sector_allocations: vec![],
            theme_exposures: vec![ThemeExposure {
                fund_code: "014002".to_string(),
                snapshot_date: date,
                theme: "AI".to_string(),
                weight_pct: 60.0,
                source: ThemeExposureSource::HoldingMapping,
            }],
            asset_allocations: vec![],
            region_allocations: vec![],
            performance: None,
            risk_metrics: None,
            announcements: vec![],
            ai_features: FundAiFeatures::default(),
            created_at: Utc::now(),
        }];

        let exposure = lookthrough_theme_exposure("p1", &positions, &snapshots);
        assert_eq!(exposure[0].exposure_value_base, 60_000.0);
        assert_eq!(exposure[0].weight_pct, 60.0);
    }

    // ── fund_top_holdings tests ──

    fn make_snapshot(
        code: &str,
        name: &str,
        holdings: Vec<FundInternalHolding>,
    ) -> FundResearchSnapshot {
        let date = NaiveDate::from_ymd_opt(2026, 6, 27).unwrap();
        FundResearchSnapshot {
            fund_code: code.to_string(),
            fund_name: name.to_string(),
            snapshot_date: date,
            source: "test".to_string(),
            quote: None,
            nav_history: vec![],
            fund_holdings: holdings,
            holding_changes: vec![],
            sector_allocations: vec![],
            theme_exposures: vec![],
            asset_allocations: vec![],
            region_allocations: vec![],
            performance: None,
            risk_metrics: None,
            announcements: vec![],
            ai_features: FundAiFeatures::default(),
            created_at: Utc::now(),
        }
    }

    fn make_holding(
        rank: Option<u32>,
        name: &str,
        weight: Option<f64>,
        code: Option<&str>,
    ) -> FundInternalHolding {
        let date = NaiveDate::from_ymd_opt(2026, 6, 27).unwrap();
        FundInternalHolding {
            fund_code: "F1".to_string(),
            report_date: date,
            rank,
            asset_code: code.map(|s| s.to_string()),
            asset_name: name.to_string(),
            asset_type: HoldingAssetType::Stock,
            market: None,
            weight_pct: weight,
            theme_tags: vec![],
        }
    }

    #[test]
    fn top_holdings_sorted_by_rank() {
        let snapshot = make_snapshot(
            "F1",
            "Fund",
            vec![
                make_holding(Some(3), "C", Some(5.0), None),
                make_holding(Some(1), "A", Some(10.0), None),
                make_holding(Some(2), "B", Some(8.0), None),
            ],
        );
        let result = fund_top_holdings(&snapshot);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].asset_name, "A");
        assert_eq!(result[1].asset_name, "B");
        assert_eq!(result[2].asset_name, "C");
    }

    #[test]
    fn top_holdings_missing_rank_stable() {
        let snapshot = make_snapshot(
            "F1",
            "Fund",
            vec![
                make_holding(None, "NoRank", Some(5.0), None),
                make_holding(Some(1), "Ranked", Some(10.0), None),
            ],
        );
        let result = fund_top_holdings(&snapshot);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].asset_name, "Ranked");
    }

    #[test]
    fn top_holdings_missing_weight_filtered() {
        let snapshot = make_snapshot(
            "F1",
            "Fund",
            vec![
                make_holding(Some(1), "Missing", None, None),
                make_holding(Some(2), "Present", Some(5.0), None),
            ],
        );
        let result = fund_top_holdings(&snapshot);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].asset_name, "Present");
    }

    #[test]
    fn top_holdings_capped_at_10() {
        let holdings: Vec<_> = (1..=15)
            .map(|i| make_holding(Some(i), &format!("H{}", i), Some(1.0), None))
            .collect();
        let snapshot = make_snapshot("F1", "Fund", holdings);
        let result = fund_top_holdings(&snapshot);
        assert_eq!(result.len(), 10);
    }

    // ── portfolio_fund_lookthrough tests ──

    #[test]
    fn lookthrough_merges_same_asset_across_funds() {
        let date = NaiveDate::from_ymd_opt(2026, 6, 27).unwrap();
        let positions = vec![
            PortfolioFundPosition {
                portfolio_id: "p1".into(),
                fund_code: "F1".into(),
                market_value_base: 100_000.0,
            },
            PortfolioFundPosition {
                portfolio_id: "p1".into(),
                fund_code: "F2".into(),
                market_value_base: 50_000.0,
            },
        ];
        let holding = |code: &str| FundInternalHolding {
            fund_code: code.into(),
            report_date: date,
            rank: Some(1),
            asset_code: Some("XYZ".into()),
            asset_name: "Same".into(),
            asset_type: HoldingAssetType::Stock,
            market: None,
            weight_pct: Some(10.0),
            theme_tags: vec![],
        };
        let snapshots = vec![
            make_snapshot("F1", "Fund1", vec![holding("F1")]),
            make_snapshot("F2", "Fund2", vec![holding("F2")]),
        ];
        let summary = portfolio_fund_lookthrough("p1", &positions, &snapshots);
        assert_eq!(summary.holdings.len(), 1);
        assert_eq!(summary.holdings[0].exposure_value_base, 15_000.0);
        assert_eq!(summary.holdings[0].weight_pct, 10.0);
        assert_eq!(summary.holdings[0].source_funds.len(), 2);
    }

    #[test]
    fn lookthrough_uses_fund_total_as_denominator() {
        let positions = vec![PortfolioFundPosition {
            portfolio_id: "p1".into(),
            fund_code: "F1".into(),
            market_value_base: 100_000.0,
        }];
        let date = NaiveDate::from_ymd_opt(2026, 6, 27).unwrap();
        let holding = FundInternalHolding {
            fund_code: "F1".into(),
            report_date: date,
            rank: Some(1),
            asset_code: Some("A".into()),
            asset_name: "A".into(),
            asset_type: HoldingAssetType::Stock,
            market: None,
            weight_pct: Some(8.0),
            theme_tags: vec![],
        };
        let snapshots = vec![make_snapshot("F1", "Fund1", vec![holding])];
        let summary = portfolio_fund_lookthrough("p1", &positions, &snapshots);
        assert_eq!(summary.total_fund_market_value_base, 100_000.0);
        assert_eq!(summary.holdings[0].exposure_value_base, 8_000.0);
        assert_eq!(summary.holdings[0].weight_pct, 8.0);
    }

    #[test]
    fn lookthrough_aggregates_by_name_when_code_missing() {
        let date = NaiveDate::from_ymd_opt(2026, 6, 27).unwrap();
        let holding = FundInternalHolding {
            fund_code: "F1".into(),
            report_date: date,
            rank: Some(1),
            asset_code: None,
            asset_name: "NoCode".into(),
            asset_type: HoldingAssetType::Stock,
            market: None,
            weight_pct: Some(10.0),
            theme_tags: vec![],
        };
        let positions = vec![PortfolioFundPosition {
            portfolio_id: "p1".into(),
            fund_code: "F1".into(),
            market_value_base: 100_000.0,
        }];
        let snapshots = vec![make_snapshot("F1", "Fund1", vec![holding.clone(), holding])];
        let summary = portfolio_fund_lookthrough("p1", &positions, &snapshots);
        assert_eq!(summary.holdings.len(), 1);
    }

    #[test]
    fn lookthrough_missing_snapshot_does_not_fail() {
        let positions = vec![
            PortfolioFundPosition {
                portfolio_id: "p1".into(),
                fund_code: "F1".into(),
                market_value_base: 100_000.0,
            },
            PortfolioFundPosition {
                portfolio_id: "p1".into(),
                fund_code: "F2".into(),
                market_value_base: 50_000.0,
            },
        ];
        let snapshots = vec![make_snapshot("F1", "Fund1", vec![])];
        let summary = portfolio_fund_lookthrough("p1", &positions, &snapshots);
        assert_eq!(summary.missing_funds, vec!["F2"]);
        assert_eq!(summary.total_fund_market_value_base, 150_000.0);
    }

    #[test]
    fn lookthrough_theme_tags_merged_deduplicated() {
        let date = NaiveDate::from_ymd_opt(2026, 6, 27).unwrap();
        let h1 = FundInternalHolding {
            fund_code: "F1".into(),
            report_date: date,
            rank: Some(1),
            asset_code: Some("A".into()),
            asset_name: "A".into(),
            asset_type: HoldingAssetType::Stock,
            market: None,
            weight_pct: Some(10.0),
            theme_tags: vec!["AI".into(), "5G".into()],
        };
        let h2 = FundInternalHolding {
            fund_code: "F2".into(),
            report_date: date,
            rank: Some(1),
            asset_code: Some("A".into()),
            asset_name: "A".into(),
            asset_type: HoldingAssetType::Stock,
            market: None,
            weight_pct: Some(5.0),
            theme_tags: vec!["AI".into(), "Chip".into()],
        };
        let positions = vec![
            PortfolioFundPosition {
                portfolio_id: "p1".into(),
                fund_code: "F1".into(),
                market_value_base: 100_000.0,
            },
            PortfolioFundPosition {
                portfolio_id: "p1".into(),
                fund_code: "F2".into(),
                market_value_base: 100_000.0,
            },
        ];
        let snapshots = vec![
            make_snapshot("F1", "F1", vec![h1]),
            make_snapshot("F2", "F2", vec![h2]),
        ];
        let summary = portfolio_fund_lookthrough("p1", &positions, &snapshots);
        let mut tags = summary.holdings[0].theme_tags.clone();
        tags.sort();
        assert_eq!(
            tags,
            vec!["5G".to_string(), "AI".to_string(), "Chip".to_string()]
        );
    }
}
