import { EastMoneyStockProvider } from "@/lib/eastmoney-stock";
import type { StockThemeProfile } from "@/lib/eastmoney-stock/types";
import { useQuery } from "@tanstack/react-query";
import type {
  HoldingClassificationOverrides,
  PortfolioFundLookthroughHolding,
  PortfolioFundLookthroughSummary,
} from "../types";
import { applyHoldingClassificationOverride, extractHoldingStockCode } from "../utils";

const provider = new EastMoneyStockProvider();

export function useEnrichedPortfolioLookthrough(
  summary: PortfolioFundLookthroughSummary | undefined,
  overrides: HoldingClassificationOverrides,
) {
  return useQuery({
    queryKey: [
      "portfolioFundLookthroughClassification",
      summary?.portfolioId,
      profileCodes(summary, overrides),
      overrides,
    ],
    queryFn: async () => {
      if (!summary) return null;

      const codes = profileCodes(summary, overrides);
      if (codes.length === 0) return applyOverrides(summary, overrides);

      const { profiles } = await provider.getStockThemeProfiles(codes);
      const profilesByCode = new Map(profiles.map((profile) => [profile.stockCode, profile]));

      return {
        ...summary,
        holdings: summary.holdings.map((holding) =>
          applyHoldingClassificationOverride(
            enrichHolding(holding, profilesByCode.get(extractHoldingStockCode(holding) ?? "")),
            overrides,
          ),
        ),
      };
    },
    enabled: !!summary,
    staleTime: 24 * 60 * 60 * 1000,
    retry: 1,
  });
}

function applyOverrides(
  summary: PortfolioFundLookthroughSummary,
  overrides: HoldingClassificationOverrides,
): PortfolioFundLookthroughSummary {
  return {
    ...summary,
    holdings: summary.holdings.map((holding) =>
      applyHoldingClassificationOverride(holding, overrides),
    ),
  };
}

function profileCodes(
  summary: PortfolioFundLookthroughSummary | undefined,
  overrides: HoldingClassificationOverrides = {},
): string[] {
  if (!summary) return [];

  const codes = new Set<string>();
  for (const holding of summary.holdings) {
    if (holding.assetType !== "stock") continue;
    const candidate = applyHoldingClassificationOverride(holding, overrides);
    if (hasCompleteClassification(candidate)) continue;

    const code = extractHoldingStockCode(holding);
    if (code) codes.add(code);
  }
  return [...codes].sort();
}

function hasCompleteClassification(holding: PortfolioFundLookthroughHolding): boolean {
  return !!(holding.sector || holding.industry) && holding.themeTags.length > 0;
}

function hasPartialClassification(holding: PortfolioFundLookthroughHolding): boolean {
  return !!(holding.sector || holding.industry) || holding.themeTags.length > 0;
}

function enrichHolding(
  holding: PortfolioFundLookthroughHolding,
  profile: StockThemeProfile | undefined,
): PortfolioFundLookthroughHolding {
  if (!profile) return holding;

  // For stocks where the profile has industry but no concepts
  // (e.g., US stocks via EastMoney extended quote with only f127 populated),
  // we still fall back to holding-level data for themes but use the
  // profile industry for sector/industry labels.
  const hasProfileConcepts = profile.concepts.length > 0;

  return {
    ...holding,
    sector: holding.sector ?? profile.industry?.boardName,
    industry: holding.industry ?? profile.industry?.boardName,
    themeTags:
      holding.themeTags.length > 0
        ? holding.themeTags
        : hasProfileConcepts
          ? profile.concepts.map((concept) => concept.name).filter(Boolean)
          : holding.themeTags,
  };
}

