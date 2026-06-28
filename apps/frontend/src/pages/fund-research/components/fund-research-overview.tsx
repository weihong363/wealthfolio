import { getAssets } from "@/adapters";
import { Button, Icons, Badge, Card, EmptyPlaceholder } from "@wealthfolio/ui";
import { Skeleton } from "@wealthfolio/ui/components/ui/skeleton";
import { useQuery } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useFundTopHoldings, useRefreshFundResearch } from "../hooks/use-fund-research";
import type { FundTopHolding } from "../types";
import type { Asset } from "@/lib/types";
import { FundTopHoldingsTable } from "./fund-top-holdings-table";

/** Assets that may have fund research data. Includes all investment-type
 * assets (stocks, ETFs, funds, etc). The backend will return empty data
 * for non-fund assets, which is handled gracefully. */
function isFundAsset(asset: Asset): boolean {
  // Investment-like assets that could be funds
  if (asset.kind !== "INVESTMENT" && asset.kind !== "PRIVATE_EQUITY") return false;
  // Exclude FX (it's treated as currency, not an investment asset)
  if (asset.kind === "FX") return false;
  // If providerConfig has a provider_id, check for fund provider
  const cfg = asset.providerConfig as Record<string, unknown> | null | undefined;
  if (cfg) {
    const pid =
      (cfg.providerId as string) ??
      (cfg.provider_id as string) ??
      (cfg.dataSource as string) ??
      "";
    if (pid && (pid.includes("FUND") || pid.includes("eastmoney") || pid.includes("Eastmoney"))) {
      return true;
    }
  }
  // For INVESTMENT kind assets without a specific fund provider check,
  // include them all — the API will just return empty for non-funds.
  return asset.kind === "INVESTMENT";
}

/** Extract the fund code (e.g. "014002") from an asset. */
function fundCode(asset: Asset): string {
  return asset.instrumentSymbol ?? asset.displayCode ?? asset.id;
}

export function FundsTab() {
  const { t } = useTranslation();
  const [selectedCode, setSelectedCode] = useState("");

  const { data: assets = [], isLoading: isLoadingAssets } = useQuery<Asset[]>({
    queryKey: ["assets"],
    queryFn: getAssets,
    staleTime: 5 * 60 * 1000,
  });

  const fundAssets = useMemo(() => assets.filter(isFundAsset), [assets]);

  const { data: holdings, isLoading, isError } = useFundTopHoldings(selectedCode || undefined);
  const refreshMutation = useRefreshFundResearch();

  const handleRefresh = () => {
    if (selectedCode) refreshMutation.mutate(selectedCode);
  };

  const top10Weight = useMemo(() => {
    if (!holdings?.length) return 0;
    return holdings.reduce((sum, h) => sum + h.weightPct, 0);
  }, [holdings]);

  const selectedName = useMemo(
    () => fundAssets.find((a) => fundCode(a) === selectedCode)?.name ?? selectedCode,
    [fundAssets, selectedCode],
  );

  // No fund assets configured
  if (!isLoadingAssets && fundAssets.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center gap-6 py-20">
        <div className="bg-muted/60 flex h-16 w-16 items-center justify-center rounded-2xl">
          <Icons.Search2 className="text-muted-foreground h-8 w-8" />
        </div>
        <div className="space-y-2 text-center">
          <p className="text-lg font-semibold">{t("fundResearch.noData")}</p>
          <p className="text-muted-foreground max-w-sm text-sm">
            {t("fundResearch.noDataDesc")}
          </p>
        </div>
      </div>
    );
  }

  // Loading assets
  if (isLoadingAssets) {
    return (
      <div className="space-y-4 p-4 md:p-6">
        <Skeleton className="h-10 w-64" />
        <Skeleton className="h-20 w-full" />
        <Skeleton className="h-64 w-full" />
      </div>
    );
  }

  return (
    <div className="space-y-4 p-4 md:p-6">
      {/* Fund Selector */}
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-2">
          <select
            value={selectedCode}
            onChange={(e) => setSelectedCode(e.target.value)}
            className="bg-background border-border h-9 min-w-[220px] rounded-lg border px-3 text-sm"
          >
            <option value="">{t("fundResearch.selectFund")}</option>
            {fundAssets.map((asset) => (
              <option key={asset.id} value={fundCode(asset)}>
                {asset.displayCode ?? asset.instrumentSymbol ?? asset.id} · {asset.name ?? asset.id}
              </option>
            ))}
          </select>
          <Button
            size="sm"
            variant="outline"
            onClick={handleRefresh}
            disabled={!selectedCode || refreshMutation.isPending}
          >
            <Icons.RefreshCw
              className={`mr-1.5 h-3.5 w-3.5 ${refreshMutation.isPending ? "animate-spin" : ""}`}
            />
            {t("fundResearch.refresh")}
          </Button>
        </div>
        {fundAssets.length > 0 && (
          <span className="text-muted-foreground text-xs">
            {t("fundResearch.holdingsCount")}: {fundAssets.length}
          </span>
        )}
      </div>

      {!selectedCode && !isLoading && (
        <div className="flex items-center justify-center py-16">
          <p className="text-muted-foreground text-sm">{t("fundResearch.selectFund")}</p>
        </div>
      )}

      {selectedCode && isLoading ? (
        <div className="space-y-3">
          <Skeleton className="h-20 w-full" />
          <Skeleton className="h-64 w-full" />
        </div>
      ) : selectedCode && isError ? (
        <EmptyPlaceholder
          icon={<Icons.AlertTriangle className="h-10 w-10" />}
          title={t("fundResearch.noData")}
          description={t("fundResearch.noDataDesc")}
        >
          <Button size="sm" onClick={handleRefresh}>
            {t("fundResearch.refreshResearch")}
          </Button>
        </EmptyPlaceholder>
      ) : selectedCode && !holdings?.length ? (
        <EmptyPlaceholder
          icon={<Icons.Search2 className="h-10 w-10" />}
          title={t("fundResearch.noData")}
          description={t("fundResearch.noDataDesc")}
        >
          <Button size="sm" onClick={handleRefresh}>
            {t("fundResearch.refreshResearch")}
          </Button>
        </EmptyPlaceholder>
      ) : selectedCode && holdings?.length ? (
        <>
          {/* Summary Strip */}
          <Card>
            <div className="grid grid-cols-2 gap-3 p-4 sm:grid-cols-4">
              <div>
                <p className="text-muted-foreground text-xs">{t("fundResearch.fund")}</p>
                <p className="truncate text-sm font-medium">
                  {holdings[0]?.fundName ?? selectedName}
                </p>
              </div>
              <div>
                <p className="text-muted-foreground text-xs">{t("fundResearch.snapshotDate")}</p>
                <p className="text-sm font-medium">{holdings[0]?.reportDate ?? "-"}</p>
              </div>
              <div>
                <p className="text-muted-foreground text-xs">{t("fundResearch.topCoverage")}</p>
                <p className="text-sm font-medium">{top10Weight.toFixed(1)}%</p>
              </div>
              <div>
                <p className="text-muted-foreground text-xs">{t("fundResearch.holdingsCount")}</p>
                <p className="text-sm font-medium">{holdings.length}</p>
              </div>
            </div>
          </Card>

          <FundTopHoldingsTable holdings={holdings} />
        </>
      ) : null}
    </div>
  );
}
