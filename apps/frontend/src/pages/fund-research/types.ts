export interface FundTopHolding {
  fundCode: string;
  fundName?: string;
  reportDate: string;
  rank?: number;
  assetCode?: string;
  assetName: string;
  assetType: string;
  market?: string;
  sector?: string;
  industry?: string;
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
  sector?: string;
  industry?: string;
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

export interface HoldingClassificationOverride {
  stockKey: string;
  assetCode?: string;
  assetName: string;
  sector?: string;
  industry?: string;
  themeTags?: string[];
  source: string;
  updatedAt: string;
}

export type HoldingClassificationOverrides = Record<string, HoldingClassificationOverride>;

export interface SaveHoldingClassificationOverrideInput {
  assetCode?: string;
  assetName: string;
  sector?: string;
  industry?: string;
  themeTags: string[];
}

export interface ThemeExposureItem {
  theme: string;
  exposureValueBase: number;
  weightPct: number;
  sourceFunds: string[];
}

export type FundResearchDistributionMode = "sector" | "theme" | "holding";

export interface HoldingClassification {
  sectors: string[];
  themes: string[];
}

export interface LookthroughDistributionSlice {
  name: string;
  exposureValueBase: number;
  weightPct: number;
}
