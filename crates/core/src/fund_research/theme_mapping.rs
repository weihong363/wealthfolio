use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use crate::errors::{Error, Result};

use super::models::{FundInternalHolding, SectorAllocation, ThemeExposure, ThemeExposureSource};

#[derive(Clone, Debug, Default, Deserialize)]
pub struct ThemeMappingConfig {
    #[serde(default)]
    pub theme_mapping: BTreeMap<String, ThemeRule>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct ThemeRule {
    #[serde(default)]
    pub sectors: Vec<String>,
    #[serde(default)]
    pub holdings: Vec<String>,
}

impl ThemeMappingConfig {
    pub fn from_toml(input: &str) -> Result<Self> {
        parse_theme_mapping_toml(input)
    }

    pub fn themes_for_holding(&self, holding: &FundInternalHolding) -> Vec<String> {
        let asset_name = holding.asset_name.to_lowercase();
        let asset_code = holding
            .asset_code
            .as_deref()
            .unwrap_or_default()
            .to_lowercase();
        let sector = holding.sector.as_deref().unwrap_or_default().to_lowercase();
        let industry = holding
            .industry
            .as_deref()
            .unwrap_or_default()
            .to_lowercase();
        self.theme_mapping
            .iter()
            .filter(|(_, rule)| {
                rule.holdings.iter().any(|name| {
                    let candidate = name.to_lowercase();
                    !candidate.is_empty()
                        && (asset_name.contains(&candidate) || asset_code == candidate)
                }) || rule.sectors.iter().any(|name| {
                    let candidate = name.to_lowercase();
                    !candidate.is_empty()
                        && (sector.contains(&candidate) || industry.contains(&candidate))
                })
            })
            .map(|(theme, _)| theme.clone())
            .collect()
    }

    pub fn themes_for_sector(&self, sector: &str) -> Vec<String> {
        let sector = sector.to_lowercase();
        self.theme_mapping
            .iter()
            .filter(|(_, rule)| {
                rule.sectors.iter().any(|candidate| {
                    let candidate = candidate.to_lowercase();
                    !candidate.is_empty() && sector.contains(&candidate)
                })
            })
            .map(|(theme, _)| theme.clone())
            .collect()
    }
}

fn parse_theme_mapping_toml(input: &str) -> Result<ThemeMappingConfig> {
    let mut config = ThemeMappingConfig::default();
    let mut current_theme: Option<String> = None;

    for raw_line in input.lines() {
        let line = raw_line.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        if let Some(theme) = line
            .strip_prefix("[theme_mapping.")
            .and_then(|line| line.strip_suffix(']'))
        {
            let theme = theme.trim().to_string();
            current_theme = Some(theme.clone());
            config.theme_mapping.entry(theme).or_default();
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            return Err(Error::InvalidConfigValue(format!(
                "Invalid theme mapping line: {line}"
            )));
        };
        let Some(theme) = current_theme.as_ref() else {
            return Err(Error::InvalidConfigValue(
                "theme_mapping entry must be inside [theme_mapping.<Theme>]".to_string(),
            ));
        };
        let values = parse_string_array(value.trim())?;
        let rule = config.theme_mapping.entry(theme.clone()).or_default();
        match key.trim() {
            "sectors" => rule.sectors = values,
            "holdings" => rule.holdings = values,
            other => {
                return Err(Error::InvalidConfigValue(format!(
                    "Unsupported theme mapping key: {other}"
                )))
            }
        }
    }

    Ok(config)
}

fn parse_string_array(value: &str) -> Result<Vec<String>> {
    let Some(inner) = value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    else {
        return Err(Error::InvalidConfigValue(format!(
            "Expected string array, got: {value}"
        )));
    };

    inner
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| {
            item.strip_prefix('"')
                .and_then(|item| item.strip_suffix('"'))
                .map(str::to_string)
                .ok_or_else(|| {
                    Error::InvalidConfigValue(format!("Expected quoted string, got: {item}"))
                })
        })
        .collect()
}

pub fn apply_theme_tags(
    holdings: &[FundInternalHolding],
    mapping: &ThemeMappingConfig,
) -> Vec<FundInternalHolding> {
    holdings
        .iter()
        .map(|holding| {
            let mut next = holding.clone();
            let mut tags: BTreeSet<String> = next.theme_tags.iter().cloned().collect();
            tags.extend(mapping.themes_for_holding(holding));
            next.theme_tags = tags.into_iter().collect();
            next
        })
        .collect()
}

pub fn calculate_theme_exposures(
    fund_code: &str,
    snapshot_date: chrono::NaiveDate,
    holdings: &[FundInternalHolding],
    sectors: &[SectorAllocation],
    mapping: &ThemeMappingConfig,
) -> Vec<ThemeExposure> {
    let mut weights: BTreeMap<(String, ThemeExposureSource), f64> = BTreeMap::new();

    for holding in apply_theme_tags(holdings, mapping) {
        let Some(weight) = holding.weight_pct else {
            continue;
        };
        for theme in holding.theme_tags {
            *weights
                .entry((theme, ThemeExposureSource::HoldingMapping))
                .or_default() += weight;
        }
    }

    for sector in sectors {
        let Some(weight) = sector.weight_pct else {
            continue;
        };
        for theme in mapping.themes_for_sector(&sector.sector) {
            *weights
                .entry((theme, ThemeExposureSource::OfficialSector))
                .or_default() += weight;
        }
    }

    weights
        .into_iter()
        .map(|((theme, source), weight_pct)| ThemeExposure {
            fund_code: fund_code.to_string(),
            snapshot_date,
            theme,
            weight_pct: weight_pct.clamp(0.0, 100.0),
            source,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::fund_research::models::HoldingAssetType;

    fn date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 6, 27).unwrap()
    }

    #[test]
    fn parses_toml_theme_mapping() {
        let mapping = ThemeMappingConfig::from_toml(
            r#"
            [theme_mapping.AI]
            sectors = ["人工智能", "半导体"]
            holdings = ["中际旭创"]
            "#,
        )
        .unwrap();
        assert_eq!(mapping.theme_mapping["AI"].holdings, vec!["中际旭创"]);
    }

    #[test]
    fn calculates_theme_exposure_from_holdings_and_sectors() {
        let mapping = ThemeMappingConfig::from_toml(
            r#"
            [theme_mapping.AI]
            sectors = ["人工智能"]
            holdings = ["中际旭创"]
            "#,
        )
        .unwrap();
        let holdings = vec![FundInternalHolding {
            fund_code: "014002".to_string(),
            report_date: date(),
            rank: Some(1),
            asset_code: None,
            asset_name: "中际旭创".to_string(),
            asset_type: HoldingAssetType::Stock,
            market: Some("A-share".to_string()),
            sector: None,
            industry: None,
            weight_pct: Some(8.0),
            theme_tags: vec![],
        }];
        let sectors = vec![SectorAllocation {
            fund_code: "014002".to_string(),
            snapshot_date: date(),
            sector: "人工智能".to_string(),
            weight_pct: Some(12.0),
            previous_weight_pct: None,
            weight_change_pct: None,
        }];

        let exposures = calculate_theme_exposures("014002", date(), &holdings, &sectors, &mapping);
        let total: f64 = exposures.iter().map(|exposure| exposure.weight_pct).sum();
        assert_eq!(total, 20.0);
    }
}
