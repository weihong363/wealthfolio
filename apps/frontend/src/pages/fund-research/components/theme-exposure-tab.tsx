import { getAssets } from "@/adapters";
import { Badge, Card, EmptyPlaceholder, formatPercent, Icons } from "@wealthfolio/ui";
import { Skeleton } from "@wealthfolio/ui/components/ui/skeleton";
import { useQuery } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Asset } from "@/lib/types";
import { useFundThemeExposure } from "../hooks/use-fund-theme-exposure";
import { useHoldingClassificationOverrides } from "../hooks/use-holding-classification-overrides";

/** Extract the fund code from an asset. */
function fundCode(asset: Asset): string {
  return asset.instrumentSymbol ?? asset.displayCode ?? asset.id;
}

export function ThemeExposureTab() {
  const { t } = useTranslation();
  const [selectedCode, setSelectedCode] = useState("");
  const { overrides } = useHoldingClassificationOverrides();

  const { data: assets = [], isLoading: isLoadingAssets } = useQuery<Asset[]>({
    queryKey: ["assets"],
    queryFn: getAssets,
    staleTime: 5 * 60 * 1000,
  });

  const fundAssets = useMemo(() => assets.filter((a) => a.kind === "INVESTMENT"), [assets]);

  const { data: exposure, isLoading, isError } = useFundThemeExposure(
    selectedCode || undefined,
    overrides,
  );

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

  if (fundAssets.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center gap-6 py-20">
        <div className="bg-muted/60 flex h-16 w-16 items-center justify-center rounded-2xl">
          <Icons.Insight className="text-muted-foreground h-8 w-8" />
        </div>
        <p className="text-muted-foreground text-sm">{t("fundResearch.themeExposureDesc")}</p>
      </div>
    );
  }

  return (
    <div className="space-y-4 p-4 md:p-6">
      {/* Fund Selector */}
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
      </div>

      {!selectedCode && (
        <div className="flex items-center justify-center py-16">
          <p className="text-muted-foreground text-sm">{t("fundResearch.selectFund")}</p>
        </div>
      )}

      {selectedCode && isLoading && (
        <div className="space-y-3">
          <Skeleton className="h-44 w-full" />
          <Skeleton className="h-56 w-full" />
        </div>
      )}

      {selectedCode && isError && (
        <EmptyPlaceholder
          icon={<Icons.AlertTriangle className="h-10 w-10" />}
          title={t("fundResearch.noData")}
          description={t("fundResearch.themeExposureDesc")}
        />
      )}

      {selectedCode && exposure && (
        <>
          {/* Summary */}
          <Card>
            <div className="grid grid-cols-2 gap-3 p-4 sm:grid-cols-4">
              <div>
                <p className="text-muted-foreground text-xs">{t("fundResearch.stockCount")}</p>
                <p className="text-sm font-medium">{exposure.holdings.length}</p>
              </div>
              <div>
                <p className="text-muted-foreground text-xs">{t("fundResearch.hotThemeWeight")}</p>
                <p className="text-sm font-medium">
                  {formatPercent(exposure.hotThemeWeight / 100)}
                </p>
              </div>
              <div>
                <p className="text-muted-foreground text-xs">{t("fundResearch.industryCount")}</p>
                <p className="text-sm font-medium">{exposure.industryDistribution.length}</p>
              </div>
              <div>
                <p className="text-muted-foreground text-xs">{t("fundResearch.conceptCount")}</p>
                <p className="text-sm font-medium">{exposure.conceptDistribution.length}</p>
              </div>
            </div>
          </Card>

          {/* Dominant Themes */}
          {exposure.dominantThemes.length > 0 && (
            <Card className="p-4">
              <p className="text-muted-foreground mb-2 text-xs font-medium">
                {t("fundResearch.dominantThemes")}
              </p>
              <div className="flex flex-wrap gap-2">
                {exposure.dominantThemes.map((theme) => (
                  <Badge key={theme} variant="secondary">
                    {theme}
                  </Badge>
                ))}
              </div>
            </Card>
          )}

          {/* Industry Distribution */}
          <Card className="p-4">
            <p className="text-muted-foreground mb-3 text-xs font-medium">
              {t("fundResearch.industryDistribution")}
            </p>
            <div className="space-y-2">
              {exposure.industryDistribution.slice(0, 10).map((item) => (
                <div
                  key={item.industry}
                  className="grid grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-3 text-sm"
                >
                  <span className="truncate">{item.industry}</span>
                  <span className="text-muted-foreground text-xs">
                    {item.stockCount} {t("fundResearch.stocks")}
                  </span>
                  <span className="w-14 text-right font-mono text-xs">
                    {formatPercent(item.weight / 100)}
                  </span>
                </div>
              ))}
            </div>
          </Card>

          {/* Concept Distribution */}
          <Card className="p-4">
            <p className="text-muted-foreground mb-3 text-xs font-medium">
              {t("fundResearch.conceptDistribution")}
            </p>
            <div className="space-y-2">
              {exposure.conceptDistribution.slice(0, 15).map((item) => (
                <div
                  key={item.concept}
                  className="grid grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-3 text-sm"
                >
                  <span className="truncate">{item.concept}</span>
                  <span className="text-muted-foreground text-xs">
                    {item.stockCount} {t("fundResearch.stocks")}
                  </span>
                  <span className="w-14 text-right font-mono text-xs">
                    {item.avgMomentumScore !== undefined
                      ? `${item.avgMomentumScore > 0 ? "+" : ""}${item.avgMomentumScore.toFixed(1)}%`
                      : "-"}
                  </span>
                </div>
              ))}
            </div>
          </Card>

          {/* Holdings detail */}
          <Card className="p-4">
            <p className="text-muted-foreground mb-3 text-xs font-medium">
              {t("fundResearch.holdingsTheme")}
            </p>
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b text-left">
                    <th className="text-muted-foreground px-2 py-1 text-xs font-medium">
                      {t("fundResearch.stock")}
                    </th>
                    <th className="text-muted-foreground px-2 py-1 text-right text-xs font-medium">
                      {t("fundResearch.weight")}
                    </th>
                    <th className="text-muted-foreground hidden px-2 py-1 text-xs font-medium sm:table-cell">
                      {t("fundResearch.industry")}
                    </th>
                    <th className="text-muted-foreground hidden px-2 py-1 text-xs font-medium md:table-cell">
                      {t("fundResearch.dominantTheme")}
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {exposure.holdings.map((h, i) => (
                    <tr key={`${h.stockCode}-${i}`} className="hover:bg-muted/30 border-b">
                      <td className="max-w-[140px] truncate px-2 py-1">
                        <span className="font-medium">{h.stockName}</span>
                        <span className="text-muted-foreground ml-1 text-xs">{h.stockCode}</span>
                      </td>
                      <td className="px-2 py-1 text-right font-mono text-xs">
                        {h.weight !== undefined ? `${h.weight.toFixed(1)}%` : "-"}
                      </td>
                      <td className="text-muted-foreground hidden truncate px-2 py-1 text-xs sm:table-cell">
                        {h.industry ?? "-"}
                      </td>
                      <td className="hidden px-2 py-1 md:table-cell">
                        {h.dominantTheme ? (
                          <Badge variant="secondary" className="text-[11px]">
                            {h.dominantTheme}
                          </Badge>
                        ) : (
                          <span className="text-muted-foreground text-xs">-</span>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Card>
        </>
      )}
    </div>
  );
}
