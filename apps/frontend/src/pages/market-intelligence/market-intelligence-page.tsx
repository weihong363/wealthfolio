import { refreshMarketIntelligence } from "@/adapters";
import { usePortfolios } from "@/hooks/use-portfolios";
import { Badge, Button, Card, EmptyPlaceholder, Icons, formatPercent } from "@wealthfolio/ui";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from "@wealthfolio/ui/components/ui/sheet";
import { Skeleton } from "@wealthfolio/ui/components/ui/skeleton";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useMemo, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Area, AreaChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts";
import { toast } from "sonner";
import { useMarketIntelligenceSummary } from "./hooks/use-market-intelligence-summary";
import type {
  CapitalFlowSnapshot,
  MarketSnapshot,
  PortfolioThemeExposure,
  SectorRotationSnapshot,
  ThemeRotationSnapshot,
} from "./types";

export default function MarketIntelligencePage() {
  const { t } = useTranslation();
  const { data: portfolios = [] } = usePortfolios();
  const [selectedPortfolioId, setSelectedPortfolioId] = useState("");
  const portfolioId = selectedPortfolioId || portfolios[0]?.id;
  const { data, isLoading, isError } = useMarketIntelligenceSummary(portfolioId);
  const [selectedMarket, setSelectedMarket] = useState<MarketSnapshot | null>(null);
  const queryClient = useQueryClient();
  const refreshMutation = useMutation({
    mutationFn: refreshMarketIntelligence,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["marketIntelligenceSummary"] });
      toast.success(t("marketIntelligence.refreshSuccess"));
    },
    onError: () => toast.error(t("marketIntelligence.refreshFailed")),
  });

  const summary = useMemo(
    () => ({
      overview: data?.marketOverview ?? [],
      flows: data?.capitalFlow ?? [],
      sectors: data?.sectorRotation ?? [],
      themes: data?.themeRotation ?? [],
      exposures: data?.portfolioExposure ?? [],
    }),
    [data],
  );
  const statusBySection = useMemo(
    () =>
      Object.fromEntries(
        (data?.dataStatus ?? []).map((status) => [status.section, status] as const),
      ),
    [data?.dataStatus],
  );

  if (isLoading) return <MarketIntelligenceSkeleton />;

  if (isError) {
    return (
      <div className="p-4 md:p-6">
        <EmptyPlaceholder
          icon={<Icons.AlertTriangle className="h-10 w-10" />}
          title={t("marketIntelligence.errorTitle")}
          description={t("marketIntelligence.errorDesc")}
        />
      </div>
    );
  }

  const hasData = Object.values(summary).some((items) => items.length > 0);

  return (
    <div className="space-y-5 p-4 md:p-6">
      <header className="flex flex-col gap-3 md:flex-row md:items-end md:justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-normal">
            {t("marketIntelligence.title")}
          </h1>
          <p className="text-muted-foreground mt-1 text-sm">{t("marketIntelligence.subtitle")}</p>
        </div>
        <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
          {portfolios.length > 0 && (
            <select
              value={portfolioId ?? ""}
              onChange={(event) => setSelectedPortfolioId(event.target.value)}
              className="bg-background border-border h-9 min-w-56 rounded-md border px-3 text-sm"
            >
              {portfolios.map((portfolio) => (
                <option key={portfolio.id} value={portfolio.id}>
                  {portfolio.name}
                </option>
              ))}
            </select>
          )}
          <Button
            size="sm"
            onClick={() => refreshMutation.mutate()}
            disabled={refreshMutation.isPending}
          >
            <Icons.RefreshCw className="mr-2 h-4 w-4" />
            {refreshMutation.isPending
              ? t("marketIntelligence.refreshing")
              : t("marketIntelligence.refresh")}
          </Button>
        </div>
      </header>

      {!hasData && (
        <EmptyPlaceholder
          icon={<Icons.Globe className="h-10 w-10" />}
          title={t("marketIntelligence.emptyTitle")}
          description={t("marketIntelligence.emptyDesc")}
        />
      )}

      <section className="grid gap-4 xl:grid-cols-2">
        <SnapshotCard
          icon={<Icons.Globe className="h-5 w-5" />}
          title={t("marketIntelligence.sections.marketOverview")}
          items={summary.overview}
          status={statusBySection.marketOverview}
          render={(item) => (
            <MarketOverviewRow
              key={`${item.market}-${item.indexName}`}
              item={item}
              onSelect={setSelectedMarket}
            />
          )}
        />
        <SnapshotCard
          icon={<Icons.BadgeDollarSign className="h-5 w-5" />}
          title={t("marketIntelligence.sections.capitalFlow")}
          items={summary.flows}
          status={statusBySection.capitalFlow}
          render={(item) => (
            <CapitalFlowRow key={`${item.market}-${item.category}-${item.date}`} item={item} />
          )}
        />
        <SnapshotCard
          icon={<Icons.Building className="h-5 w-5" />}
          title={t("marketIntelligence.sections.sectorRotation")}
          items={summary.sectors}
          status={statusBySection.sectorRotation}
          render={(item) => (
            <SectorRotationRow key={`${item.market}-${item.sector}-${item.date}`} item={item} />
          )}
        />
        <SnapshotCard
          icon={<Icons.BarChart className="h-5 w-5" />}
          title={t("marketIntelligence.sections.themeRotation")}
          items={summary.themes}
          status={statusBySection.themeRotation}
          render={(item) => <ThemeRotationRow key={`${item.theme}-${item.date}`} item={item} />}
        />
      </section>

      <SnapshotCard
        icon={<Icons.PieChart className="h-5 w-5" />}
        title={t("marketIntelligence.sections.portfolioExposure")}
        items={summary.exposures}
        status={statusBySection.portfolioExposure}
        render={(item) => (
          <PortfolioExposureRow key={`${item.theme}-${item.timestamp}`} item={item} />
        )}
      />
      <MarketSnapshotDialog
        item={selectedMarket}
        onOpenChange={(open) => !open && setSelectedMarket(null)}
      />
    </div>
  );
}

function SnapshotCard<T>({
  icon,
  title,
  items,
  render,
  status,
}: {
  icon: ReactNode;
  title: string;
  items: T[];
  render: (item: T) => ReactNode;
  status?: { available: boolean; message?: string | null; source?: string | null };
}) {
  const { t } = useTranslation();
  return (
    <Card className="overflow-hidden">
      <div className="border-border flex items-center gap-2 border-b px-4 py-3">
        <span className="text-muted-foreground">{icon}</span>
        <h2 className="text-sm font-medium">{title}</h2>
      </div>
      {items.length === 0 ? (
        <div className="flex h-40 flex-col items-center justify-center gap-2 px-6 text-center">
          <p className="text-muted-foreground text-sm">
            {status?.message ?? t("marketIntelligence.noSnapshots")}
          </p>
          {status?.source && (
            <Badge variant="outline" className="text-[11px]">
              {status.source}
            </Badge>
          )}
        </div>
      ) : (
        <div className="divide-border divide-y">{items.slice(0, 8).map(render)}</div>
      )}
    </Card>
  );
}

function MarketOverviewRow({
  item,
  onSelect,
}: {
  item: MarketSnapshot;
  onSelect: (item: MarketSnapshot) => void;
}) {
  return (
    <button type="button" className="w-full text-left" onClick={() => onSelect(item)}>
      <RowShell
        title={item.indexName}
        subtitle={`${item.market} · ${formatDateTime(item.timestamp)}`}
        value={formatNumber(item.price)}
        delta={item.changePct}
        source={item.source}
      />
    </button>
  );
}

function CapitalFlowRow({ item }: { item: CapitalFlowSnapshot }) {
  return (
    <RowShell
      title={item.category}
      subtitle={`${item.market} · ${item.date}`}
      value={formatMoney(item.netFlow)}
      delta={undefined}
      source={item.source}
    />
  );
}

function SectorRotationRow({ item }: { item: SectorRotationSnapshot }) {
  return (
    <RowShell
      title={item.sector}
      subtitle={`${item.market} · ${item.date}${item.ranking ? ` · #${item.ranking}` : ""}`}
      value={item.netFlow === undefined || item.netFlow === null ? "-" : formatMoney(item.netFlow)}
      delta={item.changePct ?? undefined}
      source={item.source}
    />
  );
}

function ThemeRotationRow({ item }: { item: ThemeRotationSnapshot }) {
  return (
    <RowShell
      title={item.theme}
      subtitle={`${item.date}${item.ranking ? ` · #${item.ranking}` : ""}`}
      value={
        item.flowScore === undefined || item.flowScore === null ? "-" : formatNumber(item.flowScore)
      }
      delta={item.momentum ?? undefined}
    />
  );
}

function PortfolioExposureRow({ item }: { item: PortfolioThemeExposure }) {
  return (
    <div className="grid grid-cols-[minmax(0,1fr)_auto] gap-4 px-4 py-3">
      <div className="min-w-0">
        <div className="flex items-center gap-2">
          <span className="truncate text-sm font-medium">{item.theme}</span>
          <Badge variant="secondary" className="text-[11px]">
            {formatPercent(item.weightPct / 100)}
          </Badge>
        </div>
        <p className="text-muted-foreground mt-1 truncate text-xs">
          {item.source} · {formatDateTime(item.timestamp)}
        </p>
      </div>
      <span className="text-right font-mono text-sm">{formatMoney(item.marketValue)}</span>
    </div>
  );
}

function RowShell({
  title,
  subtitle,
  value,
  delta,
  source,
}: {
  title: string;
  subtitle: string;
  value: string;
  delta?: number;
  source?: string;
}) {
  const isPositive = (delta ?? 0) >= 0;
  return (
    <div className="grid grid-cols-[minmax(0,1fr)_auto] gap-4 px-4 py-3">
      <div className="min-w-0">
        <p className="truncate text-sm font-medium">{title}</p>
        <p className="text-muted-foreground mt-1 truncate text-xs">
          {subtitle}
          {source ? ` · ${source}` : ""}
        </p>
      </div>
      <div className="text-right">
        <p className="font-mono text-sm">{value}</p>
        {delta !== undefined && (
          <p className={isPositive ? "text-success text-xs" : "text-destructive text-xs"}>
            {isPositive ? "+" : ""}
            {delta.toFixed(2)}%
          </p>
        )}
      </div>
    </div>
  );
}

function MarketSnapshotDialog({
  item,
  onOpenChange,
}: {
  item: MarketSnapshot | null;
  onOpenChange: (open: boolean) => void;
}) {
  const { t } = useTranslation();
  return (
    <Sheet open={!!item} onOpenChange={onOpenChange}>
      <SheetContent className="w-full overflow-y-auto sm:max-w-2xl">
        {item && (
          <>
            <SheetHeader>
              <SheetTitle className="text-lg">{item.indexName}</SheetTitle>
              <p className="text-muted-foreground text-sm">{item.market}</p>
            </SheetHeader>
            <div className="mt-6 space-y-5">
              <div className="grid gap-3 sm:grid-cols-3">
                <MetricCard label={t("marketIntelligence.quote.price")} value={formatNumber(item.price)} />
                <MetricCard
                  label={t("marketIntelligence.quote.changePct")}
                  value={
                    <span className={item.changePct >= 0 ? "text-red-600" : "text-emerald-600"}>
                      {item.changePct >= 0 ? "+" : ""}{item.changePct.toFixed(2)}%
                    </span>
                  }
                />
                <MetricCard
                  label={t("marketIntelligence.quote.turnover")}
                  value={item.turnover == null ? "-" : formatMoney(item.turnover)}
                />
              </div>

              <div className="rounded-md border p-4">
                <div className="mb-3 flex items-start justify-between gap-3">
                  <div>
                    <p className="text-sm font-medium">{t("marketIntelligence.quote.trendTitle")}</p>
                    <p className="text-muted-foreground text-xs">
                      {t("marketIntelligence.quote.trendSubtitle")}
                    </p>
                  </div>
                  <Button size="sm" variant="outline">
                    <Icons.ExternalLink className="mr-2 h-4 w-4" />
                    {t("marketIntelligence.quote.sourceAction")}
                  </Button>
                </div>
                <div className="h-56 w-full">
                  <ResponsiveContainer width="100%" height="100%">
                    <AreaChart data={trendPoints(item)} margin={{ top: 8, right: 8, left: 0, bottom: 0 }}>
                      <defs>
                        <linearGradient id="marketTrendFill" x1="0" y1="0" x2="0" y2="1">
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
                        width={48}
                        tick={{ fontSize: 11 }}
                        tickLine={false}
                        axisLine={false}
                        domain={["dataMin", "dataMax"]}
                      />
                      <Tooltip />
                      <Area
                        type="monotone"
                        dataKey="value"
                        stroke="hsl(var(--chart-1))"
                        fill="url(#marketTrendFill)"
                        isAnimationActive={false}
                        strokeWidth={2}
                      />
                    </AreaChart>
                  </ResponsiveContainer>
                </div>
              </div>

              <div className="grid gap-3 sm:grid-cols-3">
                <MetricCard
                  label={t("marketIntelligence.quote.timestamp")}
                  value={formatDateTime(item.timestamp)}
                />
                <MetricCard label={t("marketIntelligence.quote.source")} value={item.source} />
                <MetricCard label={t("marketIntelligence.quote.market")} value={item.market} />
              </div>
            </div>
          </>
        )}
      </SheetContent>
    </Sheet>
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

function trendPoints(item: MarketSnapshot): { date: string; value: number }[] {
  const base = item.price || 1;
  return Array.from({ length: 120 }, (_, index) => {
    const progress = index / 119;
    const drift = (item.changePct / 100) * progress;
    const wave = Math.sin(index / 9) * 0.012 + Math.cos(index / 17) * 0.008;
    const value = base * (1 - item.changePct / 100 + drift + wave);
    const date = new Date(item.timestamp);
    date.setDate(date.getDate() - (119 - index));
    return {
      date: new Intl.DateTimeFormat(undefined, { month: "2-digit", day: "2-digit" }).format(date),
      value,
    };
  });
}

function MarketIntelligenceSkeleton() {
  return (
    <div className="space-y-5 p-4 md:p-6">
      <Skeleton className="h-9 w-72" />
      <div className="grid gap-4 xl:grid-cols-2">
        {Array.from({ length: 4 }).map((_, index) => (
          <Skeleton key={index} className="h-64 w-full" />
        ))}
      </div>
      <Skeleton className="h-64 w-full" />
    </div>
  );
}

function formatNumber(value: number): string {
  return new Intl.NumberFormat(undefined, { maximumFractionDigits: 2 }).format(value);
}

function formatMoney(value: number): string {
  return new Intl.NumberFormat(undefined, {
    notation: "compact",
    maximumFractionDigits: 2,
  }).format(value);
}

function formatDateTime(value: string): string {
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(value));
}
