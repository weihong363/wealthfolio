export interface FundTopHolding {
  fundCode: string;
  fundName?: string;
  reportDate: string;
  rank?: number;
  assetCode?: string;
  assetName: string;
  assetType: string;
  market?: string;
  weightPct: number;
  themeTags: string[];
}

export interface FundLookthroughContribution {
  fundCode: string;
  fundName?: string;
  fundMarketValueBase: number;
  fundHoldingWeightPct: number;
  exposureValueBase: number;
  reportDate: string;
}

export interface PortfolioFundLookthroughHolding {
  assetCode?: string;
  assetName: string;
  assetType: string;
  market?: string;
  themeTags: string[];
  exposureValueBase: number;
  weightPct: number;
  sourceFunds: FundLookthroughContribution[];
}

export interface PortfolioFundLookthroughSummary {
  portfolioId: string;
  totalFundMarketValueBase: number;
  holdings: PortfolioFundLookthroughHolding[];
  missingFunds: string[];
}

export interface ThemeExposureItem {
  theme: string;
  exposureValueBase: number;
  weightPct: number;
  sourceFunds: string[];
}
