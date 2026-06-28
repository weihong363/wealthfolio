// Fund Research Commands
import type {
  FundTopHolding,
  PortfolioFundLookthroughSummary,
  ThemeExposureItem,
} from "@/pages/fund-research/types";

import { invoke, logger } from "./platform";

export const getFundTopHoldings = async (fundCode: string): Promise<FundTopHolding[]> => {
  try {
    return await invoke<FundTopHolding[]>("get_fund_top_holdings", { fundCode });
  } catch (error) {
    logger.error("Error fetching fund top holdings.");
    throw error;
  }
};

export const getPortfolioFundLookthrough = async (
  portfolioId: string,
): Promise<PortfolioFundLookthroughSummary> => {
  try {
    return await invoke<PortfolioFundLookthroughSummary>("get_portfolio_fund_lookthrough", {
      portfolioId,
    });
  } catch (error) {
    logger.error("Error fetching portfolio fund lookthrough.");
    throw error;
  }
};

export const refreshFundResearch = async (fundCode: string): Promise<void> => {
  try {
    await invoke<void>("refresh_fund_research", { fundCode });
  } catch (error) {
    logger.error("Error refreshing fund research.");
    throw error;
  }
};

export const getPortfolioThemeExposure = async (
  portfolioId: string,
): Promise<ThemeExposureItem[]> => {
  try {
    return await invoke<ThemeExposureItem[]>("get_portfolio_theme_exposure", { portfolioId });
  } catch (error) {
    logger.error("Error fetching portfolio theme exposure.");
    throw error;
  }
};
