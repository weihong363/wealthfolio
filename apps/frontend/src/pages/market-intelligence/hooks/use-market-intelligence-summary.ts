import { getMarketIntelligenceSummary } from "@/adapters";
import { useQuery } from "@tanstack/react-query";

export function useMarketIntelligenceSummary(portfolioId?: string) {
  return useQuery({
    queryKey: ["marketIntelligenceSummary", portfolioId ?? "none"],
    queryFn: () => getMarketIntelligenceSummary(portfolioId),
    staleTime: 60_000,
  });
}
