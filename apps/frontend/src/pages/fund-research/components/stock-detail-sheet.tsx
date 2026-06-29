import { EastMoneyStockProvider, normalizeStockCode } from "@/lib/eastmoney-stock";
import { stockPageUrl } from "@/lib/eastmoney-stock/endpoints";
import type { StockPricePoint, StockThemeProfile } from "@/lib/eastmoney-stock/types";
import { AmountDisplay, Badge, Button, formatPercent, Icons } from "@wealthfolio/ui";
import { Sheet, SheetContent, SheetHeader, SheetTitle } from "@wealthfolio/ui/components/ui/sheet";
import { Skeleton } from "@wealthfolio/ui/components/ui/skeleton";
import { useQuery } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { Area, AreaChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts";
import { useTranslation } from "react-i18next";
import type { PortfolioFundLookthroughHolding } from "../types";
import { cleanAssetName, extractHoldingStockCode } from "../utils";

const provider = new EastMoneyStockProvider();

export function StockDetailSheet({
  holding,
  open,
  onOpenChange,
}: {
  holding: PortfolioFundLookthroughHolding | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { t } = useTranslation();
  const stockCode = holding ? extractHoldingStockCode(holding) : null;
  const normalized = stockCode ? normalizeStockCode(stockCode) : null;

  const profileQuery = useQuery({
    queryKey: ["fundResearchStockProfile", stockCode],
    queryFn: () => provider.getStockThemeProfile(stockCode!),
    enabled: open && !!stockCode,
    staleTime: 3 * 60 * 1000,
    retry: 1,
  });

  const historyQuery = useQuery({
    queryKey: ["fundResearchStockHistory", stockCode],
    queryFn: () => provider.getStockPriceHistory(stockCode!, 120),
    enabled: open && !!stockCode,
    staleTime: 30 * 60 * 1000,
    retry: 1,
  });

  if (!holding) return null;

  const profile = profileQuery.data;
  const industry = profile?.industry?.boardName ?? holding.industry ?? holding.sector;
  const themes = profile?.concepts.map((concept) => concept.name) ?? holding.themeTags;
  const quoteUrl = normalized ? stockPageUrl(normalized.code, normalized.exchange) : undefined;
  const title = stockTitle(profile?.stockName, cleanAssetName(holding.assetName), stockCode);

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent className="w-full overflow-y-auto sm:max-w-2xl">
        <SheetHeader>
          <SheetTitle className="text-lg">{title}</SheetTitle>
          <p className="text-muted-foreground text-sm">
            {profile?.exchange ?? normalized?.exchange ?? t("fundResearch.noStockCode")}
          </p>
        </SheetHeader>

        <div className="mt-6 space-y-5">
          {!stockCode ? (
            <div className="text-muted-foreground rounded-md border p-4 text-sm">
              {t("fundResearch.noStockCode")}
            </div>
          ) : (
            <>
              <StockQuotePanel profile={profile} isLoading={profileQuery.isLoading} />
              {profileQuery.isError && (
                <div className="text-muted-foreground rounded-md border p-3 text-xs">
                  {t("fundResearch.stockQuoteLoadFailed")}
                </div>
              )}

              <div className="rounded-md border p-4">
                <div className="mb-3 flex items-center justify-between gap-3">
                  <div>
                    <p className="text-sm font-medium">{t("fundResearch.stockTrend")}</p>
                    <p className="text-muted-foreground text-xs">
                      {t("fundResearch.stockTrendDesc")}
                    </p>
                  </div>
                  {quoteUrl && (
                    <Button asChild size="sm" variant="outline">
                      <a href={quoteUrl} target="_blank" rel="noreferrer">
                        <Icons.ExternalLink className="mr-2 h-4 w-4" />
                        {t("fundResearch.eastmoneyQuote")}
                      </a>
                    </Button>
                  )}
                </div>
                <StockTrendChart
                  data={historyQuery.data ?? []}
                  isLoading={historyQuery.isLoading}
                />
              </div>

              <div className="grid gap-3 sm:grid-cols-3">
                <MetricCard
                  label={t("fundResearch.lookthroughExposure")}
                  value={<AmountDisplay value={holding.exposureValueBase} currency="CNY" />}
                />
                <MetricCard
                  label={t("fundResearch.weight")}
                  value={formatPercent(holding.weightPct / 100)}
                />
                <MetricCard
                  label={t("fundResearch.sourceFunds")}
                  value={t("fundResearch.fundsCount", { count: holding.sourceFunds.length })}
                />
              </div>

              <div className="rounded-md border p-4">
                <p className="mb-3 text-sm font-medium">{t("fundResearch.stockProfile")}</p>
                <div className="space-y-3">
                  <LabelRow label={t("fundResearch.industry")}>
                    {industry ? <Badge variant="secondary">{industry}</Badge> : t("fundResearch.unclassified")}
                  </LabelRow>
                  <LabelRow label={t("fundResearch.themes")}>
                    <div className="flex flex-wrap gap-1">
                      {(themes.length > 0 ? themes : [t("fundResearch.unclassified")]).map((theme) => (
                        <Badge key={theme} variant="secondary" className="max-w-40 truncate">
                          {theme}
                        </Badge>
                      ))}
                    </div>
                  </LabelRow>
                  {profile?.marketCap !== undefined && (
                    <LabelRow label={t("fundResearch.marketCap")}>
                      {formatCompactNumber(profile.marketCap)}
                    </LabelRow>
                  )}
                </div>
              </div>

              <div className="rounded-md border p-4">
                <p className="mb-3 text-sm font-medium">{t("fundResearch.sourceFunds")}</p>
                <div className="space-y-2">
                  {holding.sourceFunds.map((sourceFund) => (
                    <div
                      key={sourceFund.fundCode}
                      className="bg-muted/30 grid grid-cols-[1fr_auto] gap-2 rounded-md p-3 text-sm"
                    >
                      <div>
                        <p className="font-medium">{sourceFund.fundName ?? sourceFund.fundCode}</p>
                        <p className="text-muted-foreground text-xs">{sourceFund.fundCode}</p>
                      </div>
                      <div className="text-right text-xs">
                        <p>{sourceFund.fundHoldingWeightPct.toFixed(2)}%</p>
                        <AmountDisplay value={sourceFund.exposureValueBase} currency="CNY" />
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </>
          )}
        </div>
      </SheetContent>
    </Sheet>
  );
}

function StockQuotePanel({
  profile,
  isLoading,
}: {
  profile: StockThemeProfile | undefined;
  isLoading: boolean;
}) {
  const { t } = useTranslation();
  if (isLoading) {
    return (
      <div className="grid gap-3 sm:grid-cols-3">
        <MetricLoadingCard label={t("fundResearch.latestPrice")} />
        <MetricLoadingCard label={t("fundResearch.dailyChange")} />
        <MetricLoadingCard label={t("fundResearch.turnover")} />
      </div>
    );
  }

  return (
    <div className="grid gap-3 sm:grid-cols-3">
      <MetricCard label={t("fundResearch.latestPrice")} value={formatPrice(profile?.latestPrice)} />
      <MetricCard
        label={t("fundResearch.dailyChange")}
        value={
          <span className={changeClass(profile?.changePercent)}>
            {formatRawPercent(profile?.changePercent)}
          </span>
        }
      />
      <MetricCard label={t("fundResearch.turnover")} value={formatCompactNumber(profile?.amount)} />
    </div>
  );
}

function StockTrendChart({ data, isLoading }: { data: StockPricePoint[]; isLoading: boolean }) {
  const { t } = useTranslation();
  if (isLoading) return <Skeleton className="h-56 w-full" />;
  if (data.length === 0) {
    return (
      <div className="text-muted-foreground flex h-56 items-center justify-center text-sm">
        {t("fundResearch.noTrendData")}
      </div>
    );
  }
  const closes = data.map((point) => point.close).filter(isNumber);
  if (closes.length === 0) {
    return (
      <div className="text-muted-foreground flex h-56 items-center justify-center text-sm">
        {t("fundResearch.noTrendData")}
      </div>
    );
  }
  const minClose = Math.min(...closes);
  const maxClose = Math.max(...closes);
  const padding = Math.max((maxClose - minClose) * 0.08, maxClose * 0.005, 0.01);

  return (
    <div className="h-56 w-full">
      <ResponsiveContainer width="100%" height="100%">
        <AreaChart data={data} margin={{ top: 8, right: 8, left: 0, bottom: 0 }}>
          <defs>
            <linearGradient id="stockPriceGradient" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="hsl(var(--chart-1))" stopOpacity={0.24} />
              <stop offset="100%" stopColor="hsl(var(--chart-1))" stopOpacity={0} />
            </linearGradient>
          </defs>
          <XAxis
            dataKey="date"
            tick={{ fontSize: 11 }}
            minTickGap={28}
            tickLine={false}
            axisLine={false}
          />
          <YAxis
            domain={[minClose - padding, maxClose + padding]}
            tick={{ fontSize: 11 }}
            tickLine={false}
            axisLine={false}
            width={48}
          />
          <Tooltip
            formatter={(value) => [formatPrice(Number(value)), t("fundResearch.closePrice")]}
            labelFormatter={(label) => String(label)}
          />
          <Area
            type="monotone"
            dataKey="close"
            stroke="hsl(var(--chart-1))"
            fill="url(#stockPriceGradient)"
            dot={false}
            isAnimationActive={false}
            strokeWidth={2}
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}

function MetricCard({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div className="rounded-md border p-3">
      <p className="text-muted-foreground text-xs">{label}</p>
      <div className="mt-1 text-sm font-medium">{value}</div>
    </div>
  );
}

function MetricLoadingCard({ label }: { label: string }) {
  return (
    <div className="rounded-md border p-3">
      <p className="text-muted-foreground text-xs">{label}</p>
      <Skeleton className="mt-2 h-5 w-20" />
    </div>
  );
}

function LabelRow({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="grid gap-2 text-sm sm:grid-cols-[96px_1fr]">
      <span className="text-muted-foreground">{label}</span>
      <div>{children}</div>
    </div>
  );
}

function formatPrice(value: number | undefined): string {
  if (value === undefined) return "-";
  return new Intl.NumberFormat("zh-CN", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(value);
}

function stockTitle(
  profileName: string | undefined,
  fallbackName: string,
  stockCode: string | null,
): string {
  const name = profileName?.trim() || fallbackName || stockCode || "";
  return stockCode ? `${name}(${stockCode})` : name;
}

function formatRawPercent(value: number | undefined): string {
  if (value === undefined) return "-";
  return `${value >= 0 ? "+" : ""}${value.toFixed(2)}%`;
}

function formatCompactNumber(value: number | undefined): string {
  if (value === undefined) return "-";
  return new Intl.NumberFormat("zh-CN", {
    notation: "compact",
    maximumFractionDigits: 2,
  }).format(value);
}

function changeClass(value: number | undefined): string {
  if (value === undefined || value === 0) return "";
  return value > 0 ? "text-red-600" : "text-emerald-600";
}

function isNumber(value: number | undefined): value is number {
  return typeof value === "number" && Number.isFinite(value);
}
