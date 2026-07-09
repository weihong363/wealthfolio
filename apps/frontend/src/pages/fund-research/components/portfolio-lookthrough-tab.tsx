import {
  AmountDisplay,
  Badge,
  Button,
  Card,
  EmptyPlaceholder,
  formatPercent,
  Icons,
} from "@wealthfolio/ui";
import { MetricLabelWithInfo } from "@/components/metric-display";
import { Skeleton } from "@wealthfolio/ui/components/ui/skeleton";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { usePortfolioFundLookthrough } from "../hooks/use-fund-research";
import { useEnrichedPortfolioLookthrough } from "../hooks/use-enriched-portfolio-lookthrough";
import { useHoldingClassificationOverrides } from "../hooks/use-holding-classification-overrides";
import { HoldingClassificationSheet } from "./holding-classification-sheet";
import { SourceFundsSheet } from "./source-funds-sheet";
import { StockDetailSheet } from "./stock-detail-sheet";
import type { FundResearchDistributionMode, PortfolioFundLookthroughHolding } from "../types";
import {
  buildLookthroughDistribution,
  classifyHolding,
  cleanAssetName,
  holdingClassificationKey,
} from "../utils";
import { LookthroughDistributionChart } from "./lookthrough-distribution-chart";

export function PortfolioLookthroughTab() {
  const { t } = useTranslation();
  const [selectedHolding, setSelectedHolding] = useState<PortfolioFundLookthroughHolding | null>(
    null,
  );
  const [classificationHolding, setClassificationHolding] =
    useState<PortfolioFundLookthroughHolding | null>(null);
  const [stockDetailHolding, setStockDetailHolding] =
    useState<PortfolioFundLookthroughHolding | null>(null);
  const [topN, setTopN] = useState(20);
  const [distributionMode, setDistributionMode] = useState<FundResearchDistributionMode>("sector");
  const { overrides, saveOverride, clearOverride } = useHoldingClassificationOverrides();

  const { data: summary, isLoading, isError } = usePortfolioFundLookthrough("default");
  const { data: enrichedSummary } = useEnrichedPortfolioLookthrough(summary, overrides);
  const activeSummary = enrichedSummary ?? summary;

  if (isLoading) {
    return (
      <div className="space-y-3 p-4 md:p-6">
        <Skeleton className="h-20 w-full" />
        <Skeleton className="h-64 w-full" />
      </div>
    );
  }

  if (isError || !activeSummary) {
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

  const displayedHoldings =
    topN > 0 ? activeSummary.holdings.slice(0, topN) : activeSummary.holdings;
  const coveredValue = activeSummary.holdings.reduce((s, h) => s + h.exposureValueBase, 0);
  const distribution = buildLookthroughDistribution(
    activeSummary.holdings,
    distributionMode,
    t("fundResearch.unclassified"),
  );

  return (
    <div className="space-y-4 p-4 md:p-6">
      {/* Summary Strip */}
      <Card>
        <div className="grid grid-cols-2 gap-3 p-4 sm:grid-cols-4">
          <div>
            <p className="text-muted-foreground text-xs">{t("fundResearch.totalFundValue")}</p>
            <p className="text-sm font-medium">
              <AmountDisplay value={activeSummary.totalFundMarketValueBase} currency="CNY" />
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
              {activeSummary.totalFundMarketValueBase > 0
                ? ((coveredValue / activeSummary.totalFundMarketValueBase) * 100).toFixed(1)
                : "0"}
              %
            </p>
          </div>
          <div>
            <p className="text-muted-foreground text-xs">{t("fundResearch.underlyingCount")}</p>
            <p className="text-sm font-medium">{activeSummary.holdings.length}</p>
          </div>
        </div>
      </Card>

      {/* Missing Snapshots Warning */}
      {activeSummary.missingFunds.length > 0 && (
        <div className="border-warning/20 bg-warning/10 flex items-center gap-2 rounded-lg border p-3 text-sm">
          <Icons.AlertTriangle className="text-warning h-4 w-4 shrink-0" />
          <span>
            {t("fundResearch.missingSnapshotsWarning")}: {activeSummary.missingFunds.join(", ")}
          </span>
        </div>
      )}

      <LookthroughDistributionChart
        mode={distributionMode}
        onModeChange={setDistributionMode}
        data={distribution}
      />

      {/* Top N Selector */}
      <div className="flex items-center gap-2">
        <MetricLabelWithInfo label="" infoText={t("fundResearch.sectionsInfo.lookthrough")} />
        <span className="text-muted-foreground text-xs">{t("fundResearch.underlyingHolding")}</span>
        {[10, 20, 50, 0].map((n) => (
          <Button
            key={n}
            size="xs"
            variant={topN === n ? "default" : "outline"}
            onClick={() => setTopN(n)}
            className="h-7 text-xs"
          >
            {n === 0 ? t("fundResearch.all") : t("fundResearch.topN", { count: n })}
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
              <th className="text-muted-foreground hidden px-2 py-2 text-xs font-medium lg:table-cell">
                {t("fundResearch.sector")}
              </th>
              <th className="text-muted-foreground hidden px-2 py-2 text-xs font-medium xl:table-cell">
                {t("fundResearch.themes")}
              </th>
              <th className="text-muted-foreground px-2 py-2 text-right text-xs font-medium">
                {t("fundResearch.actions")}
              </th>
            </tr>
          </thead>
          <tbody>
            {displayedHoldings.map((h, i) => {
              const classification = classifyHolding(h);
              return (
                <tr
                  key={`${h.assetCode ?? h.assetName}-${i}`}
                  className="hover:bg-muted/30 border-b"
                >
                  <td className="max-w-[160px] px-2 py-2 font-medium">
                    <button
                      type="button"
                      className="hover:text-primary max-w-full truncate text-left underline-offset-2 hover:underline"
                      onClick={() => setStockDetailHolding(h)}
                    >
                      {cleanAssetName(h.assetName)}
                    </button>
                  </td>
                  <td className="px-2 py-2 text-right text-xs tabular-nums">
                    <AmountDisplay value={h.exposureValueBase} currency="CNY" />
                  </td>
                  <td className="px-2 py-2 text-right font-mono text-xs">
                    {formatPercent(h.weightPct / 100)}
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
                  <td className="hidden max-w-[180px] px-2 py-2 lg:table-cell">
                    <ClassificationBadges
                      labels={classification.sectors}
                      fallback={t("fundResearch.unclassified")}
                      limit={3}
                    />
                  </td>
                  <td className="hidden max-w-[220px] px-2 py-2 xl:table-cell">
                    <ClassificationBadges
                      labels={classification.themes}
                      fallback={t("fundResearch.unclassified")}
                    />
                  </td>
                  <td className="px-2 py-2 text-right">
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-7 w-7"
                      title={t("fundResearch.editClassification")}
                      onClick={() => setClassificationHolding(h)}
                    >
                      <Icons.Pencil className="h-3.5 w-3.5" />
                    </Button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      {/* Source Funds Sheet */}
      <SourceFundsSheet
        holding={selectedHolding}
        open={!!selectedHolding}
        onOpenChange={(open) => !open && setSelectedHolding(null)}
      />
      <StockDetailSheet
        holding={stockDetailHolding}
        open={!!stockDetailHolding}
        onOpenChange={(open) => !open && setStockDetailHolding(null)}
      />
      <HoldingClassificationSheet
        holding={classificationHolding}
        override={
          classificationHolding
            ? overrides[holdingClassificationKey(classificationHolding)]
            : undefined
        }
        open={!!classificationHolding}
        onOpenChange={(open) => !open && setClassificationHolding(null)}
        onSave={saveOverride}
        onClear={clearOverride}
      />
    </div>
  );
}

function ClassificationBadges({
  labels,
  fallback,
  limit,
}: {
  labels: string[];
  fallback: string;
  limit?: number;
}) {
  const displayLabels = labels.length > 0 ? labels.slice(0, limit) : [fallback];

  return (
    <div className="flex flex-wrap gap-1">
      {displayLabels.map((label) => (
        <Badge key={label} variant="secondary" className="max-w-28 truncate text-[11px]">
          {label}
        </Badge>
      ))}
    </div>
  );
}
