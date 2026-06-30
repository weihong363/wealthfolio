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
  dataStatus: MarketIntelligenceDataStatus[];
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
