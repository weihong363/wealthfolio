export interface MarketSnapshot {
  market: string;
  indexName: string;
  price: number;
  changePct: number;
  turnover?: number | null;
  timestamp: string;
  source: string;
}

export interface CapitalFlowSnapshot {
  market: string;
  date: string;
  category: string;
  inflow?: number | null;
  outflow?: number | null;
  netFlow: number;
  source: string;
}

export interface SectorRotationSnapshot {
  market: string;
  sector: string;
  date: string;
  netFlow?: number | null;
  changePct?: number | null;
  turnover?: number | null;
  ranking?: number | null;
  source: string;
}

export interface ThemeRotationSnapshot {
  theme: string;
  date: string;
  flowScore?: number | null;
  momentum?: number | null;
  ranking?: number | null;
}

export interface PortfolioThemeExposure {
  portfolioId: string;
  theme: string;
  weightPct: number;
  marketValue: number;
  source: string;
  timestamp: string;
}

export interface MarketIntelligenceSummary {
  marketOverview: MarketSnapshot[];
  capitalFlow: CapitalFlowSnapshot[];
  sectorRotation: SectorRotationSnapshot[];
  themeRotation: ThemeRotationSnapshot[];
  portfolioExposure: PortfolioThemeExposure[];
  trends: MarketIntelligenceTrends;
  dataStatus: MarketIntelligenceDataStatus[];
}

export interface MarketIntelligenceTrends {
  granularity: string;
  windows: MarketIntelligenceTrendWindow[];
}

export interface MarketIntelligenceTrendWindow {
  window: string;
  days?: number | null;
  capitalFlow: FlowTrend;
  sectorRotation: FlowTrend;
  themeRotation: ThemeTrend;
}

export interface FlowTrend {
  series: NamedTimeSeries[];
  topInflows: RankingItem[];
  topOutflows: RankingItem[];
}

export interface ThemeTrend {
  flowScoreSeries: NamedTimeSeries[];
  momentumSeries: NamedTimeSeries[];
  topThemes: RankingItem[];
  bottomThemes: RankingItem[];
}

export interface NamedTimeSeries {
  name: string;
  market?: string | null;
  points: TimeSeriesPoint[];
  latestValue?: number | null;
  cumulativeValue: number;
  source?: string | null;
}

export interface TimeSeriesPoint {
  date: string;
  value: number;
  cumulative: number;
}

export interface RankingItem {
  name: string;
  market?: string | null;
  value: number;
  secondaryValue?: number | null;
  ranking?: number | null;
  date?: string | null;
  source?: string | null;
}

export interface MarketIntelligenceDataStatus {
  section: string;
  available: boolean;
  message?: string | null;
  source?: string | null;
}

export interface BrowserMarketIntelligenceSnapshots {
  marketOverview: MarketSnapshot[];
  capitalFlow: CapitalFlowSnapshot[];
  sectorRotation: SectorRotationSnapshot[];
}
