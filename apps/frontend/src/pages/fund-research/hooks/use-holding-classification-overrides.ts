import {
  deleteStockClassificationOverride,
  getStockClassificationOverrides,
  saveStockClassificationOverride,
} from "@/adapters";
import { QueryKeys } from "@/lib/query-keys";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useMemo } from "react";
import type { HoldingClassificationOverrides, PortfolioFundLookthroughHolding } from "../types";
import { cleanAssetName, extractHoldingStockCode, holdingClassificationKey } from "../utils";

export function useHoldingClassificationOverrides() {
  const queryClient = useQueryClient();
  const overridesQuery = useQuery({
    queryKey: [QueryKeys.STOCK_CLASSIFICATION_OVERRIDES],
    queryFn: getStockClassificationOverrides,
    staleTime: 5 * 60 * 1000,
  });

  const overrides = useMemo<HoldingClassificationOverrides>(() => {
    const result: HoldingClassificationOverrides = {};
    for (const override of overridesQuery.data ?? []) {
      result[override.stockKey] = override;
    }
    return result;
  }, [overridesQuery.data]);

  const invalidateFundResearch = useCallback(() => {
    queryClient.invalidateQueries({ queryKey: [QueryKeys.STOCK_CLASSIFICATION_OVERRIDES] });
    queryClient.invalidateQueries({ queryKey: [QueryKeys.FUND_TOP_HOLDINGS] });
    queryClient.invalidateQueries({ queryKey: [QueryKeys.PORTFOLIO_FUND_LOOKTHROUGH] });
    queryClient.invalidateQueries({ queryKey: [QueryKeys.PORTFOLIO_THEME_EXPOSURE] });
    queryClient.invalidateQueries({ queryKey: ["portfolioFundLookthroughClassification"] });
  }, [queryClient]);

  const saveMutation = useMutation({
    mutationFn: ({
      holding,
      input,
    }: {
      holding: PortfolioFundLookthroughHolding;
      input: { sector?: string; industry?: string; themeTags?: string[] };
    }) =>
      saveStockClassificationOverride({
        assetCode: extractHoldingStockCode(holding) ?? undefined,
        assetName: cleanAssetName(holding.assetName),
        sector: input.sector,
        industry: input.industry,
        themeTags: input.themeTags ?? [],
      }),
    onSuccess: invalidateFundResearch,
  });
  const { mutate: saveClassificationOverride, isPending: isSavePending } = saveMutation;

  const deleteMutation = useMutation({
    mutationFn: (holding: PortfolioFundLookthroughHolding) =>
      deleteStockClassificationOverride(holdingClassificationKey(holding)),
    onSuccess: invalidateFundResearch,
  });
  const { mutate: deleteClassificationOverride, isPending: isDeletePending } = deleteMutation;

  const saveOverride = useCallback(
    (
      holding: PortfolioFundLookthroughHolding,
      input: { sector?: string; industry?: string; themeTags?: string[] },
    ) => {
      saveClassificationOverride({ holding, input });
    },
    [saveClassificationOverride],
  );

  const clearOverride = useCallback(
    (holding: PortfolioFundLookthroughHolding) => {
      deleteClassificationOverride(holding);
    },
    [deleteClassificationOverride],
  );

  return useMemo(
    () => ({
      overrides,
      saveOverride,
      clearOverride,
      isLoading: overridesQuery.isLoading,
      isSaving: isSavePending || isDeletePending,
    }),
    [
      clearOverride,
      isDeletePending,
      isSavePending,
      overrides,
      overridesQuery.isLoading,
      saveOverride,
    ],
  );
}
