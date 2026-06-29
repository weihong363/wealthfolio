// ─── Core stock profile types ───────────────────────────────────────────────

export interface StockThemeProfile {
  stockCode: string;
  stockName?: string;
  exchange?: Exchange;
  secid?: string;
  latestPrice?: number;
  changePercent?: number;
  turnover?: number;
  volume?: number;
  amount?: number;
  marketCap?: number;
  industry?: StockIndustryInfo;
  concepts: StockConceptInfo[];
  boards: StockBoardInfo[];
  metrics: StockThemeMetrics;
  source: "eastmoney";
  sourceUrls: string[];
  fetchedAt: string;
}

export interface StockPricePoint {
  date: string;
  open?: number;
  close?: number;
  high?: number;
  low?: number;
  volume?: number;
  amount?: number;
  changePercent?: number;
}

export type Exchange = "SH" | "SZ" | "BJ" | "US" | "HK" | "UNKNOWN";

// ─── Industry ────────────────────────────────────────────────────────────────

export interface StockIndustryInfo {
  boardCode?: string;
  boardName?: string;
  level?: number;
  changePercent?: number;
  mainNetInflow?: number;
  amount?: number;
  rankInBoard?: number;
}

// ─── Concepts / Themes ───────────────────────────────────────────────────────

export interface StockConceptInfo {
  code?: string;
  name: string;
  changePercent?: number;
  mainNetInflow?: number;
  amount?: number;
  heatRank?: number;
  stockRankInConcept?: number;
  isHot?: boolean;
  sourceUrl?: string;
}

// ─── Boards ──────────────────────────────────────────────────────────────────

export interface StockBoardInfo {
  code?: string;
  name: string;
  type: BoardType;
  changePercent?: number;
  mainNetInflow?: number;
  amount?: number;
}

export type BoardType = "industry" | "concept" | "region" | "style" | "unknown";

// ─── Derived metrics ─────────────────────────────────────────────────────────

export interface StockThemeMetrics {
  conceptCount: number;
  hotConceptCount: number;
  dominantTheme?: string;
  themeExposureScore?: number;
  industryMomentumScore?: number;
  conceptMomentumScore?: number;
  topConceptsByHeat: string[];
  topConceptsByMainInflow: string[];
  fundRelevanceTags: string[];
}

// ─── Fund theme exposure (fund holdings → stock themes) ──────────────────────

export interface FundThemeExposure {
  fundCode: string;
  fundName?: string;
  holdings: FundHoldingTheme[];
  industryDistribution: IndustryDistributionItem[];
  conceptDistribution: ConceptDistributionItem[];
  dominantThemes: string[];
  hotThemeWeight: number;
  fetchedAt: string;
}

export interface FundHoldingTheme {
  stockCode: string;
  stockName: string;
  weight?: number;
  industry?: string;
  concepts: string[];
  dominantTheme?: string;
  themeExposureScore?: number;
}

export interface IndustryDistributionItem {
  industry: string;
  weight: number;
  stockCount: number;
}

export interface ConceptDistributionItem {
  concept: string;
  weight: number;
  stockCount: number;
  avgMomentumScore?: number;
}

// ─── Batch result ────────────────────────────────────────────────────────────

export interface BatchStockProfileResult {
  profiles: StockThemeProfile[];
  errors: { stockCode: string; message: string }[];
}
