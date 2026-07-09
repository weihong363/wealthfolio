import type {
  BrowserMarketIntelligenceSnapshots,
  MarketIntelligenceSummary,
} from "@/pages/market-intelligence/types";
import { fetchEastmoneyMarketIntelligenceSnapshots } from "./eastmoney-market-intelligence";
import { invoke, isWeb, logger } from "./platform";

export const getMarketIntelligenceSummary = async (
  portfolioId?: string,
  window?: string,
): Promise<MarketIntelligenceSummary> => {
  try {
    return await invoke<MarketIntelligenceSummary>("get_market_intelligence_summary", {
      portfolioId,
      window,
    });
  } catch (error) {
    logger.error("Error fetching market intelligence summary.");
    throw error;
  }
};

export const refreshMarketIntelligence = async (): Promise<MarketIntelligenceSummary> => {
  if (isWeb) {
    try {
      const snapshots = await fetchEastmoneyMarketIntelligenceSnapshots();
      if (hasBrowserSnapshots(snapshots)) {
        return await ingestMarketIntelligenceSnapshots(snapshots);
      }
    } catch (error) {
      logger.warn("Browser Eastmoney refresh failed; falling back to backend refresh.", error);
    }
  }

  try {
    return await invoke<MarketIntelligenceSummary>("refresh_market_intelligence");
  } catch (error) {
    logger.error("Error refreshing market intelligence.");
    throw error;
  }
};

export const ingestMarketIntelligenceSnapshots = async (
  snapshots: BrowserMarketIntelligenceSnapshots,
): Promise<MarketIntelligenceSummary> => {
  return await invoke<MarketIntelligenceSummary>("ingest_market_intelligence_snapshots", {
    snapshots,
  });
};

function hasBrowserSnapshots(snapshots: BrowserMarketIntelligenceSnapshots): boolean {
  return (
    snapshots.marketOverview.length > 0 ||
    snapshots.capitalFlow.length > 0 ||
    snapshots.sectorRotation.length > 0
  );
}
