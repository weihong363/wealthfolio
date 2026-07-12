use chrono::Utc;

use super::super::macro_capital::models::indicator;
use super::models::{flow_state, FlowSignal, FlowSignalInputs, MarketLiquidityMetrics};

/// Turnover change (percent) above which liquidity is considered "improving".
const TURNOVER_UP_PCT: f64 = 5.0;
/// Turnover change (percent) below which liquidity is considered "contracting".
const TURNOVER_DOWN_PCT: f64 = -5.0;
/// Sector inflow share above which breadth is considered broad-based.
const BREADTH_STRONG: f64 = 0.55;
/// Sector inflow share below which breadth is considered weak.
const BREADTH_WEAK: f64 = 0.45;
/// |net/gross| below which aggregate money is considered roughly flat.
const NET_RATIO_FLAT: f64 = 0.15;

/// Rule-based (non-AI) market flow-signal analyzer.
///
/// It consumes objective inputs already assembled elsewhere (northbound /
/// southbound flow, margin balance, market turnover, sector and theme rotation)
/// and classifies the current money regime. It performs no data collection, no
/// persistence, and never produces trade advice.
#[derive(Default)]
pub struct MarketFlowSignalService;

impl MarketFlowSignalService {
    pub fn new() -> Self {
        Self
    }

    /// Classify the current market money-flow state from objective inputs.
    ///
    /// Rules:
    /// - `new_money`: liquidity improving (turnover up or northbound inflow)
    ///   AND most sectors flowing in together.
    /// - `rotation`: aggregate money roughly flat but clear sector
    ///   inflow/outflow substitution.
    /// - `outflow`: macro liquidity falling AND most sectors flowing out.
    /// - `neutral`: data present but no decisive direction.
    /// - `unknown`: not enough data (never fabricated).
    pub fn evaluate(&self, inputs: &FlowSignalInputs<'_>) -> FlowSignal {
        let liquidity = build_liquidity(inputs);
        let breadth = sector_breadth(inputs);
        let latest_macro = |ind: &str| {
            inputs
                .macro_capital
                .iter()
                .filter(|s| s.indicator == ind)
                .max_by_key(|s| s.date)
                .map(|s| s.value)
        };
        let northbound = latest_macro(indicator::NORTHBOUND);
        let margin_balance = latest_macro(indicator::MARGIN_BALANCE);

        let sources = collect_sources(inputs);

        // Not enough data to judge anything -> Unknown. We require at least
        // sector breadth OR a macro-liquidity signal (turnover / northbound).
        let has_breadth = breadth.is_some();
        let has_liquidity_signal = liquidity.turnover_change_pct.is_some() || northbound.is_some();
        if !has_breadth && !has_liquidity_signal {
            return FlowSignal {
                state: flow_state::UNKNOWN.to_string(),
                confidence: 0.0,
                evidence: vec![
                    "缺少板块广度与市场流动性数据，数据不足，无法判断资金状态。".to_string()
                ],
                timestamp: Utc::now(),
                sources,
                data_complete: false,
                liquidity,
            };
        }

        let mut evidence = Vec::new();

        // Liquidity direction: prefer turnover change, fall back to northbound.
        let liquidity_direction = liquidity_direction(&liquidity, northbound, &mut evidence);
        // Breadth direction and dispersion.
        let breadth = breadth.map(|b| {
            evidence.push(format!(
                "板块净流入占比 {:.0}%（{} 流入 / {} 流出），净额/总额比 {:+.0}%。",
                b.inflow_share * 100.0,
                b.inflow_sectors,
                b.outflow_sectors,
                b.net_ratio * 100.0,
            ));
            b
        });

        if let Some(north) = northbound {
            evidence.push(format!("北向资金净流入 {north:+.2} 亿元。"));
        }
        if let Some(margin) = margin_balance {
            evidence.push(format!("两融余额 {margin:.0} 亿元。"));
        }

        let (state, confidence) = classify(liquidity_direction, breadth.as_ref(), &mut evidence);

        // data_complete requires breadth, turnover change and northbound flow.
        let data_complete =
            breadth.is_some() && liquidity.turnover_change_pct.is_some() && northbound.is_some();

        FlowSignal {
            state: state.to_string(),
            confidence,
            evidence,
            timestamp: Utc::now(),
            sources,
            data_complete,
            liquidity,
        }
    }
}

/// Coarse liquidity direction: +1 improving, -1 contracting, 0 flat/unknown.
#[derive(Clone, Copy, PartialEq)]
enum LiquidityDirection {
    Improving,
    Contracting,
    Flat,
}

fn liquidity_direction(
    liquidity: &MarketLiquidityMetrics,
    northbound: Option<f64>,
    evidence: &mut Vec<String>,
) -> LiquidityDirection {
    if let Some(change) = liquidity.turnover_change_pct {
        if let Some(turnover) = liquidity.total_turnover {
            evidence.push(format!(
                "两市成交额 {:.0} 亿元，较上一交易日 {:+.1}%。",
                turnover, change
            ));
        } else {
            evidence.push(format!("两市成交额较上一交易日 {change:+.1}%。"));
        }
        if change >= TURNOVER_UP_PCT {
            return LiquidityDirection::Improving;
        }
        if change <= TURNOVER_DOWN_PCT {
            return LiquidityDirection::Contracting;
        }
        return LiquidityDirection::Flat;
    }

    // No turnover change available; fall back to northbound flow sign.
    match northbound {
        Some(north) if north > 0.0 => LiquidityDirection::Improving,
        Some(north) if north < 0.0 => LiquidityDirection::Contracting,
        _ => LiquidityDirection::Flat,
    }
}

struct SectorBreadth {
    inflow_sectors: i32,
    outflow_sectors: i32,
    inflow_share: f64,
    net_ratio: f64,
}

fn sector_breadth(inputs: &FlowSignalInputs<'_>) -> Option<SectorBreadth> {
    let latest_date = inputs.sector_rotation.iter().map(|s| s.date).max()?;
    let mut inflow_sectors = 0;
    let mut outflow_sectors = 0;
    let mut net = 0.0;
    let mut gross = 0.0;
    for snapshot in inputs
        .sector_rotation
        .iter()
        .filter(|s| s.date == latest_date)
    {
        let Some(value) = snapshot.net_flow.or(snapshot.change_pct) else {
            continue;
        };
        if value > 0.0 {
            inflow_sectors += 1;
        } else if value < 0.0 {
            outflow_sectors += 1;
        }
        net += value;
        gross += value.abs();
    }
    let total = inflow_sectors + outflow_sectors;
    if total == 0 || gross == 0.0 {
        return None;
    }
    Some(SectorBreadth {
        inflow_sectors,
        outflow_sectors,
        inflow_share: inflow_sectors as f64 / total as f64,
        net_ratio: net / gross,
    })
}

fn classify(
    liquidity: LiquidityDirection,
    breadth: Option<&SectorBreadth>,
    evidence: &mut Vec<String>,
) -> (&'static str, f64) {
    match breadth {
        Some(b) => {
            let broad_in = b.inflow_share >= BREADTH_STRONG;
            let broad_out = b.inflow_share <= BREADTH_WEAK;
            let flat_money = b.net_ratio.abs() < NET_RATIO_FLAT;
            let dispersed = (0.30..=0.70).contains(&b.inflow_share);

            match liquidity {
                LiquidityDirection::Improving if broad_in => {
                    evidence
                        .push("市场流动性改善且多数板块同步流入，判定为增量资金进入。".to_string());
                    (flow_state::NEW_MONEY, confidence(0.9, b))
                }
                LiquidityDirection::Contracting if broad_out => {
                    evidence.push("宏观资金指标下降且多数板块流出，判定为整体流出。".to_string());
                    (flow_state::OUTFLOW, confidence(0.9, b))
                }
                _ if flat_money && dispersed => {
                    evidence.push(
                        "市场总资金变化不明显但存在明显板块流入/流出替代，判定为板块轮动。"
                            .to_string(),
                    );
                    (flow_state::ROTATION, confidence(0.7, b))
                }
                LiquidityDirection::Improving if broad_out => {
                    evidence
                        .push("流动性改善但板块普遍流出，方向背离，判定为板块轮动。".to_string());
                    (flow_state::ROTATION, confidence(0.55, b))
                }
                LiquidityDirection::Contracting if broad_in => {
                    evidence
                        .push("流动性收缩但板块普遍流入，方向背离，判定为板块轮动。".to_string());
                    (flow_state::ROTATION, confidence(0.55, b))
                }
                _ => {
                    evidence.push("资金活跃度与方向均不显著，判定为中性。".to_string());
                    (flow_state::NEUTRAL, 0.5)
                }
            }
        }
        // No breadth: judge on liquidity direction alone, lower confidence.
        None => match liquidity {
            LiquidityDirection::Improving => {
                evidence.push("仅有市场流动性数据且趋于改善，暂判定为增量资金进入。".to_string());
                (flow_state::NEW_MONEY, 0.45)
            }
            LiquidityDirection::Contracting => {
                evidence.push("仅有市场流动性数据且趋于收缩，暂判定为整体流出。".to_string());
                (flow_state::OUTFLOW, 0.45)
            }
            LiquidityDirection::Flat => {
                evidence.push("市场流动性变化不明显，判定为中性。".to_string());
                (flow_state::NEUTRAL, 0.4)
            }
        },
    }
}

/// Confidence scales with how far the sector inflow share is from the 50%
/// balance point, capped at `base`.
fn confidence(base: f64, breadth: &SectorBreadth) -> f64 {
    let strength = ((breadth.inflow_share - 0.5).abs() * 2.0).min(1.0);
    (base * (0.7 + 0.3 * strength)).clamp(0.0, 1.0)
}

fn build_liquidity(inputs: &FlowSignalInputs<'_>) -> MarketLiquidityMetrics {
    let as_of = inputs.market_overview.iter().map(|s| s.timestamp).max();
    let total_turnover = combined_turnover(inputs);
    let turnover_change_pct = match (total_turnover, inputs.previous_turnover) {
        (Some(current), Some(previous)) if previous > 0.0 => {
            Some((current - previous) / previous * 100.0)
        }
        _ => None,
    };

    let (advancing, declining) = sector_advance_decline(inputs);
    let advance_decline_ratio = match (advancing, declining) {
        (Some(up), Some(down)) if down > 0 => Some(up as f64 / down as f64),
        _ => None,
    };

    MarketLiquidityMetrics {
        total_turnover,
        turnover_change_pct,
        advancing,
        declining,
        advance_decline_ratio,
        as_of: as_of.map(|ts| ts.date_naive()),
    }
}

/// Combined turnover of the CN benchmark indices (Shanghai + Shenzhen), taken
/// from the latest market-overview snapshots. Returns None when unavailable.
fn combined_turnover(inputs: &FlowSignalInputs<'_>) -> Option<f64> {
    let sum: f64 = inputs
        .market_overview
        .iter()
        .filter(|s| s.market == "CN")
        .filter_map(|s| s.turnover)
        .sum();
    (sum > 0.0).then_some(sum)
}

/// Sector-level advance/decline counts on the freshest sector date. Used as a
/// breadth proxy because individual-stock A/D counts are not collected.
fn sector_advance_decline(inputs: &FlowSignalInputs<'_>) -> (Option<i32>, Option<i32>) {
    let Some(latest_date) = inputs.sector_rotation.iter().map(|s| s.date).max() else {
        return (None, None);
    };
    let mut advancing = 0;
    let mut declining = 0;
    let mut seen = false;
    for snapshot in inputs
        .sector_rotation
        .iter()
        .filter(|s| s.date == latest_date)
    {
        if let Some(change) = snapshot.change_pct {
            seen = true;
            if change > 0.0 {
                advancing += 1;
            } else if change < 0.0 {
                declining += 1;
            }
        }
    }
    if seen {
        (Some(advancing), Some(declining))
    } else {
        (None, None)
    }
}

fn collect_sources(inputs: &FlowSignalInputs<'_>) -> Vec<String> {
    let mut sources = Vec::new();
    for s in inputs.sector_rotation {
        push_unique(&mut sources, &s.source);
    }
    for s in inputs.macro_capital {
        push_unique(&mut sources, &s.source);
    }
    for s in inputs.market_overview {
        push_unique(&mut sources, &s.source);
    }
    sources
}

fn push_unique(items: &mut Vec<String>, value: &str) {
    if !value.is_empty() && !items.iter().any(|item| item == value) {
        items.push(value.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market_intelligence::macro_capital::MacroCapitalSnapshot;
    use crate::market_intelligence::market_overview::MarketSnapshot;
    use crate::market_intelligence::sector_rotation::SectorRotationSnapshot;
    use chrono::{NaiveDate, TimeZone, Utc};

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 7, day).unwrap()
    }

    fn sector(name: &str, day: u32, net_flow: f64, change_pct: f64) -> SectorRotationSnapshot {
        SectorRotationSnapshot {
            market: "CN".to_string(),
            sector: name.to_string(),
            date: date(day),
            net_flow: Some(net_flow),
            change_pct: Some(change_pct),
            turnover: None,
            ranking: None,
            source: "eastmoney".to_string(),
        }
    }

    fn northbound(day: u32, value: f64) -> MacroCapitalSnapshot {
        MacroCapitalSnapshot {
            indicator: indicator::NORTHBOUND.to_string(),
            market: "CN".to_string(),
            date: date(day),
            value,
            change: None,
            unit: Some("100M_CNY".to_string()),
            source: "eastmoney_datacenter".to_string(),
        }
    }

    fn overview(turnover: f64) -> MarketSnapshot {
        MarketSnapshot {
            market: "CN".to_string(),
            index_name: "上证指数".to_string(),
            price: 3000.0,
            change_pct: 1.0,
            turnover: Some(turnover),
            timestamp: Utc.with_ymd_and_hms(2026, 7, 8, 7, 0, 0).unwrap(),
            source: "eastmoney".to_string(),
        }
    }

    #[test]
    fn returns_unknown_when_no_data() {
        let inputs = FlowSignalInputs {
            sector_rotation: &[],
            theme_rotation: &[],
            macro_capital: &[],
            market_overview: &[],
            previous_turnover: None,
        };
        let signal = MarketFlowSignalService::new().evaluate(&inputs);
        assert_eq!(signal.state, flow_state::UNKNOWN);
        assert_eq!(signal.confidence, 0.0);
        assert!(!signal.data_complete);
    }

    #[test]
    fn detects_new_money_when_liquidity_up_and_breadth_broad() {
        let sectors = vec![
            sector("电子", 8, 100.0, 2.0),
            sector("通信", 8, 80.0, 1.5),
            sector("计算机", 8, 60.0, 1.2),
            sector("银行", 8, -10.0, -0.3),
        ];
        let macro_capital = vec![northbound(8, 50.0)];
        let overview_rows = vec![overview(6000.0)];
        let inputs = FlowSignalInputs {
            sector_rotation: &sectors,
            theme_rotation: &[],
            macro_capital: &macro_capital,
            market_overview: &overview_rows,
            previous_turnover: Some(5000.0),
        };
        let signal = MarketFlowSignalService::new().evaluate(&inputs);
        assert_eq!(signal.state, flow_state::NEW_MONEY);
        assert!(signal.confidence > 0.0);
        assert!(signal.data_complete);
        assert_eq!(signal.liquidity.total_turnover, Some(6000.0));
        assert_eq!(signal.liquidity.turnover_change_pct, Some(20.0));
        assert_eq!(signal.liquidity.advancing, Some(3));
        assert_eq!(signal.liquidity.declining, Some(1));
    }

    #[test]
    fn detects_outflow_when_liquidity_down_and_breadth_weak() {
        let sectors = vec![
            sector("电子", 8, -100.0, -2.0),
            sector("通信", 8, -80.0, -1.5),
            sector("计算机", 8, -60.0, -1.2),
            sector("银行", 8, 5.0, 0.1),
        ];
        let macro_capital = vec![northbound(8, -40.0)];
        let overview_rows = vec![overview(4000.0)];
        let inputs = FlowSignalInputs {
            sector_rotation: &sectors,
            theme_rotation: &[],
            macro_capital: &macro_capital,
            market_overview: &overview_rows,
            previous_turnover: Some(5000.0),
        };
        let signal = MarketFlowSignalService::new().evaluate(&inputs);
        assert_eq!(signal.state, flow_state::OUTFLOW);
        assert!(signal.data_complete);
        assert_eq!(signal.liquidity.turnover_change_pct, Some(-20.0));
    }

    #[test]
    fn detects_rotation_when_money_flat_but_dispersed() {
        let sectors = vec![
            sector("电子", 8, 100.0, 2.0),
            sector("通信", 8, 90.0, 1.8),
            sector("银行", 8, -95.0, -1.9),
            sector("地产", 8, -90.0, -1.7),
        ];
        let overview_rows = vec![overview(5100.0)];
        let inputs = FlowSignalInputs {
            sector_rotation: &sectors,
            theme_rotation: &[],
            macro_capital: &[],
            market_overview: &overview_rows,
            previous_turnover: Some(5000.0),
        };
        let signal = MarketFlowSignalService::new().evaluate(&inputs);
        assert_eq!(signal.state, flow_state::ROTATION);
    }

    #[test]
    fn unknown_confidence_is_zero_and_no_fabrication() {
        let inputs = FlowSignalInputs {
            sector_rotation: &[],
            theme_rotation: &[],
            macro_capital: &[],
            market_overview: &[],
            previous_turnover: None,
        };
        let signal = MarketFlowSignalService::new().evaluate(&inputs);
        assert_eq!(signal.state, flow_state::UNKNOWN);
        assert!(signal.liquidity.total_turnover.is_none());
        assert!(signal.liquidity.advance_decline_ratio.is_none());
    }
}
