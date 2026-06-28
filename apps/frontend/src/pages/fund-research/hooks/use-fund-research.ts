import {
  getFundTopHoldings,
  getPortfolioFundLookthrough,
  getPortfolioThemeExposure,
  refreshFundResearch,
} from "@/adapters";
import { QueryKeys } from "@/lib/query-keys";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

export function useFundTopHoldings(fundCode?: string) {
  return useQuery({
    queryKey: [QueryKeys.FUND_TOP_HOLDINGS, fundCode],
    queryFn: () => getFundTopHoldings(fundCode!),
    enabled: !!fundCode,
    staleTime: 5 * 60 * 1000,
  });
}

export function usePortfolioFundLookthrough(portfolioId?: string) {
  return useQuery({
    queryKey: [QueryKeys.PORTFOLIO_FUND_LOOKTHROUGH, portfolioId],
    queryFn: () => getPortfolioFundLookthrough(portfolioId!),
    enabled: !!portfolioId,
    staleTime: 5 * 60 * 1000,
  });
}

export function usePortfolioThemeExposure(portfolioId?: string) {
  return useQuery({
    queryKey: [QueryKeys.PORTFOLIO_THEME_EXPOSURE, portfolioId],
    queryFn: () => getPortfolioThemeExposure(portfolioId!),
    enabled: !!portfolioId,
    staleTime: 5 * 60 * 1000,
  });
}

export function useRefreshFundResearch() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (fundCode: string) => refreshFundResearch(fundCode),
    onSuccess: (_data, fundCode) => {
      queryClient.invalidateQueries({ queryKey: [QueryKeys.FUND_TOP_HOLDINGS, fundCode] });
      queryClient.invalidateQueries({ queryKey: [QueryKeys.PORTFOLIO_FUND_LOOKTHROUGH] });
    },
  });
}
