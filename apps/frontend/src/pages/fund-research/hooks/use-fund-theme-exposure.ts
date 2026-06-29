import { useQuery } from "@tanstack/react-query";
import { EastMoneyStockProvider } from "@/lib/eastmoney-stock";
import { useFundTopHoldings } from "./use-fund-research";
import type { FundThemeExposure } from "@/lib/eastmoney-stock/types";
import type { HoldingClassificationOverrides } from "../types";
import { cleanAssetName, extractHoldingStockCode } from "../utils";

const provider = new EastMoneyStockProvider();

/**
 * Fetches theme exposure for a fund by enriching its top holdings
 * with Eastmoney stock industry/concept data. If classification
 * overrides are provided, the dominant themes are replaced with
 * the manually-curated theme tags from stock_classification_overrides.
 */
export function useFundThemeExposure(
  fundCode?: string,
  overrides?: HoldingClassificationOverrides,
) {
  const { data: holdings, isLoading: isLoadingHoldings } = useFundTopHoldings(
    fundCode || undefined,
  );
  const fundName = holdings?.[0]?.fundName;

  return useQuery<FundThemeExposure | null>({
    queryKey: ["fundThemeExposure", fundCode, fundName, holdings, overrides],
    queryFn: async () => {
      if (!holdings?.length || !fundCode) return null;

      const stockHoldings: { stockCode: string; stockName: string; weight?: number }[] = [];
      for (const h of holdings) {
        if (h.assetType !== "stock") continue;
        const code = extractHoldingStockCode(h);
        if (!code) continue;
        stockHoldings.push({
          stockCode: code,
          stockName: cleanAssetName(h.assetName),
          weight: h.weightPct,
        });
      }

      if (stockHoldings.length === 0) return null;

      const result = await provider.getFundThemeExposure(
        fundCode,
        fundName,
        stockHoldings,
      );

      // Apply stock classification overrides: replace each holding's
      // dominantTheme with the manually-curated theme tags so the
      // "占优主题" (dominant themes) view reflects user corrections.
      if (overrides && Object.keys(overrides).length > 0) {
        for (const h of result.holdings) {
          const stockKey = h.stockCode.toUpperCase();
          const override = overrides[stockKey];
          if (override?.themeTags?.length) {
            h.dominantTheme = override.themeTags[0];
          }
        }
        // Recompute dominantThemes from overridden holdings
        const themeSet = new Set<string>();
        for (const h of result.holdings) {
          if (h.dominantTheme) themeSet.add(h.dominantTheme);
        }
        result.dominantThemes = [...themeSet].slice(0, 10);
      }

      return result;
    },
    enabled: !!fundCode && !isLoadingHoldings && !!holdings?.length,
    staleTime: 5 * 60 * 1000,
    retry: 1,
  });
}
