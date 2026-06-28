import { AmountDisplay, Badge, Button, Card, EmptyPlaceholder, formatPercent, Icons } from "@wealthfolio/ui";
import { Skeleton } from "@wealthfolio/ui/components/ui/skeleton";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { usePortfolioFundLookthrough } from "../hooks/use-fund-research";
import { SourceFundsSheet } from "./source-funds-sheet";
import type { PortfolioFundLookthroughHolding } from "../types";

export function PortfolioLookthroughTab() {
  const { t } = useTranslation();
  const [selectedHolding, setSelectedHolding] = useState<PortfolioFundLookthroughHolding | null>(
    null,
  );
  const [topN, setTopN] = useState(20);

  const { data: summary, isLoading, isError } = usePortfolioFundLookthrough("default");

  if (isLoading) {
    return (
      <div className="space-y-3 p-4 md:p-6">
        <Skeleton className="h-20 w-full" />
        <Skeleton className="h-64 w-full" />
      </div>
    );
  }

  if (isError || !summary) {
    return (
      <div className="flex items-center justify-center py-20">
        <EmptyPlaceholder
          icon={<Icons.AlertTriangle className="h-10 w-10" />}
          title={t("fundResearch.noData")}
          description={t("fundResearch.noDataDesc")}
        />
      </div>
    );
  }

  const displayedHoldings = topN > 0 ? summary.holdings.slice(0, topN) : summary.holdings;
  const coveredValue = displayedHoldings.reduce((s, h) => s + h.exposureValueBase, 0);

  return (
    <div className="space-y-4 p-4 md:p-6">
      {/* Summary Strip */}
      <Card>
        <div className="grid grid-cols-2 gap-3 p-4 sm:grid-cols-4">
          <div>
            <p className="text-muted-foreground text-xs">{t("fundResearch.totalFundValue")}</p>
            <p className="text-sm font-medium">
              <AmountDisplay
                value={summary.totalFundMarketValueBase}
                currency="CNY"
              />
            </p>
          </div>
          <div>
            <p className="text-muted-foreground text-xs">{t("fundResearch.coveredValue")}</p>
            <p className="text-sm font-medium">
              <AmountDisplay value={coveredValue} currency="CNY" />
            </p>
          </div>
          <div>
            <p className="text-muted-foreground text-xs">{t("fundResearch.coverage")}</p>
            <p className="text-sm font-medium">
              {summary.totalFundMarketValueBase > 0
                ? ((coveredValue / summary.totalFundMarketValueBase) * 100).toFixed(1)
                : "0"}
              %
            </p>
          </div>
          <div>
            <p className="text-muted-foreground text-xs">{t("fundResearch.underlyingCount")}</p>
            <p className="text-sm font-medium">{summary.holdings.length}</p>
          </div>
        </div>
      </Card>

      {/* Missing Snapshots Warning */}
      {summary.missingFunds.length > 0 && (
        <div className="border-warning/20 bg-warning/10 flex items-center gap-2 rounded-lg border p-3 text-sm">
          <Icons.AlertTriangle className="text-warning h-4 w-4 shrink-0" />
          <span>
            {t("fundResearch.missingSnapshotsWarning")}: {summary.missingFunds.join(", ")}
          </span>
        </div>
      )}

      {/* Top N Selector */}
      <div className="flex items-center gap-2">
        <span className="text-muted-foreground text-xs">{t("fundResearch.underlyingHolding")}</span>
        {[10, 20, 50, 0].map((n) => (
          <Button
            key={n}
            size="xs"
            variant={topN === n ? "default" : "outline"}
            onClick={() => setTopN(n)}
            className="h-7 text-xs"
          >
            {n === 0 ? "All" : `Top ${n}`}
          </Button>
        ))}
      </div>

      {/* Holdings Table */}
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b text-left">
              <th className="text-muted-foreground px-2 py-2 text-xs font-medium">
                {t("fundResearch.asset")}
              </th>
              <th className="text-muted-foreground px-2 py-2 text-right text-xs font-medium">
                {t("fundResearch.exposureValue")}
              </th>
              <th className="text-muted-foreground px-2 py-2 text-right text-xs font-medium">
                {t("fundResearch.weight")}
              </th>
              <th className="text-muted-foreground hidden px-2 py-2 text-xs font-medium sm:table-cell">
                {t("fundResearch.sourceFunds")}
              </th>
              <th className="text-muted-foreground hidden px-2 py-2 text-xs font-medium md:table-cell">
                {t("fundResearch.type")}
              </th>
            </tr>
          </thead>
          <tbody>
            {displayedHoldings.map((h, i) => (
              <tr key={i} className="hover:bg-muted/30 border-b">
                <td className="max-w-[160px] truncate px-2 py-2 font-medium">
                  {h.assetName}
                </td>
                <td className="px-2 py-2 text-right text-xs tabular-nums">
                  <AmountDisplay value={h.exposureValueBase} currency="CNY" />
                </td>
                <td className="px-2 py-2 text-right font-mono text-xs">
                  {formatPercent(h.weightPct)}
                </td>
                <td className="hidden px-2 py-2 sm:table-cell">
                  <Button
                    variant="ghost"
                    size="xs"
                    onClick={() => setSelectedHolding(h)}
                    className="text-xs underline"
                  >
                    {t("fundResearch.fundsCount", { count: h.sourceFunds.length })}
                  </Button>
                </td>
                <td className="text-muted-foreground hidden px-2 py-2 text-xs capitalize md:table-cell">
                  {h.assetType}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Source Funds Sheet */}
      <SourceFundsSheet
        holding={selectedHolding}
        open={!!selectedHolding}
        onOpenChange={(open) => !open && setSelectedHolding(null)}
      />
    </div>
  );
}
