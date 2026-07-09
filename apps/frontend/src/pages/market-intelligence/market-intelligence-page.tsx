import { refreshMarketIntelligence } from "@/adapters";
import { usePortfolios } from "@/hooks/use-portfolios";
import { MetricLabelWithInfo } from "@/components/metric-display";
import { Badge, Button, Card, EmptyPlaceholder, Icons } from "@wealthfolio/ui";
import { Skeleton } from "@wealthfolio/ui/components/ui/skeleton";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  CartesianGrid,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { toast } from "sonner";
import { useMarketIntelligenceSummary } from "./hooks/use-market-intelligence-summary";
import type {
  CapitalFlowSnapshot,
  FlowTrend,
  MarketIntelligenceTrendWindow,
  NamedTimeSeries,
  RankingItem,
  SectorRotationSnapshot,
  ThemeTrend,
  ThemeRotationSnapshot,
} from "./types";

export default function MarketIntelligencePage() {
  const { t } = useTranslation();
  const { data: portfolios = [] } = usePortfolios();
  const [selectedPortfolioId, setSelectedPortfolioId] = useState("");
  const [activeWindow, setActiveWindow] = useState("20d");
  const portfolioId = selectedPortfolioId || portfolios[0]?.id;
  const { data, isLoading, isError } = useMarketIntelligenceSummary(portfolioId, activeWindow);
  const queryClient = useQueryClient();
  const refreshMutation = useMutation({
    mutationFn: refreshMarketIntelligence,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["marketIntelligenceSummary"] });
      toast.success(t("marketIntelligence.refreshSuccess"));
    },
    onError: () => toast.error(t("marketIntelligence.refreshFailed")),
  });

  const trends = useMemo(() => {
    if (hasTrendWindows(data?.trends?.windows)) {
      return data?.trends;
    }
    return buildFallbackTrends(
      data?.capitalFlow ?? [],
      data?.sectorRotation ?? [],
      data?.themeRotation ?? [],
    );
  }, [data?.capitalFlow, data?.sectorRotation, data?.themeRotation, data?.trends]);

  const summary = useMemo(
    () => ({
      overview: data?.marketOverview ?? [],
      flows: data?.capitalFlow ?? [],
      sectors: data?.sectorRotation ?? [],
      themes: data?.themeRotation ?? [],
      exposures: data?.portfolioExposure ?? [],
      trends,
    }),
    [data, trends],
  );
  const trendWindow = useMemo(() => {
    const windows = summary.trends?.windows ?? [];
    return (
      windows.find((window) => window.window === activeWindow) ??
      windows.find((window) => window.window === "20d") ??
      windows[0]
    );
  }, [activeWindow, summary.trends?.windows]);
  const mergedExposures = useMemo(() => {
    const themeNames = summary.themes.reduce((acc, snap) => {
      const key = snap.theme.toLowerCase();
      if (!acc.has(key) || snap.date > acc.get(key)!.date) {
        acc.set(key, snap);
      }
      return acc;
    }, new Map<string, ThemeRotationSnapshot>());

    const merged = summary.exposures.map((exp) => {
      const rotation = themeNames.get(exp.theme.toLowerCase());
      const momentum = rotation?.momentum ?? null;
      const weight = exp.weightPct;
      const heavy = weight >= 8;
      const hot = momentum !== null && momentum > 2;
      const cold = momentum !== null && momentum < -2;

      let assessment: "hot_heavy" | "warm" | "cold_heavy" | "neutral" = "neutral";
      if (heavy && hot) assessment = "hot_heavy";
      else if (heavy && cold) assessment = "cold_heavy";
      else if (hot) assessment = "warm";

      return { ...exp, momentum, assessment };
    });

    merged.sort((a, b) => {
      if (a.assessment === b.assessment) return b.weightPct - a.weightPct;
      const order = { hot_heavy: 0, cold_heavy: 1, warm: 2, neutral: 3 };
      return order[a.assessment] - order[b.assessment];
    });

    return { items: merged };
  }, [summary.exposures, summary.themes]);

  const statusBySection = useMemo(() => {
    const statusMap = Object.fromEntries(
      (data?.dataStatus ?? []).map((status) => [status.section, status] as const),
    );
    // Override with actual data presence (may come from browser JSONP)
    if (summary.flows.length > 0) {
      statusMap.capitalFlow = {
        section: "capitalFlow",
        available: true,
        source: "eastmoney_browser",
      };
    }
    if (summary.sectors.length > 0) {
      statusMap.sectorRotation = {
        section: "sectorRotation",
        available: true,
        source: "eastmoney_browser",
      };
    }
    if (summary.themes.length > 0) {
      statusMap.themeRotation = {
        section: "themeRotation",
        available: true,
        source: "eastmoney_browser",
      };
    }
    return statusMap;
  }, [data?.dataStatus, summary.flows.length, summary.sectors.length, summary.themes.length]);

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

  const hasData =
    summary.overview.length > 0 ||
    summary.flows.length > 0 ||
    summary.sectors.length > 0 ||
    summary.themes.length > 0 ||
    summary.exposures.length > 0;

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

      <TrendDashboard
        activeWindow={trendWindow}
        windows={summary.trends?.windows ?? []}
        selectedWindow={activeWindow}
        onWindowChange={setActiveWindow}
      />

      <MarketBreadthPanel
        sectors={summary.sectors}
        themes={summary.themes}
        days={trendWindow?.days ?? null}
      />

      <PortfolioExposureCard
        items={mergedExposures.items}
        status={statusBySection.portfolioExposure}
      />
    </div>
  );
}

function TrendDashboard({
  activeWindow,
  windows,
  selectedWindow,
  onWindowChange,
}: {
  activeWindow?: MarketIntelligenceTrendWindow;
  windows: MarketIntelligenceTrendWindow[];
  selectedWindow: string;
  onWindowChange: (window: string) => void;
}) {
  const { t } = useTranslation();
  const [mode, setMode] = useState<"sector" | "theme">("sector");
  const hasTrendData =
    !!activeWindow &&
    (activeWindow.sectorRotation.series.length > 0 ||
      activeWindow.themeRotation.flowScoreSeries.length > 0);
  const rankingTrend =
    mode === "sector" ? activeWindow?.sectorRotation : themeRankingAsFlow(activeWindow?.themeRotation);
  const chartTrend = normalizeReplayTrend(
    mode === "sector" ? activeWindow?.sectorRotation : themeTrendAsFlow(activeWindow?.themeRotation),
  );
  const rankingInflows = rankingTrend?.topInflows ?? [];
  const rankingOutflows = rankingTrend?.topOutflows ?? [];
  const hasPositiveMoneyRankings = rankingInflows.some((item) => item.value > 0);
  const hasNegativeMoneyRankings = rankingOutflows.some((item) => item.value < 0);
  const displayInflows = hasPositiveMoneyRankings ? rankingInflows : (chartTrend?.topInflows ?? []);
  const displayOutflows = hasNegativeMoneyRankings ? rankingOutflows : (chartTrend?.topOutflows ?? []);
  const inflowValueKind: RankingValueKind = hasPositiveMoneyRankings ? "money" : "percent";
  const outflowValueKind: RankingValueKind = hasNegativeMoneyRankings ? "money" : "percent";
  const chartSeries = chartTrend?.series ?? [];
  const colorByKey = buildSeriesColorMap(chartSeries, "mixed", 30);

  return (
    <section className="space-y-3">
      <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
        <div>
          <h2 className="text-base font-semibold tracking-normal">
            {t("marketIntelligence.trends.title")}
          </h2>
          <p className="text-muted-foreground mt-1 text-sm">
            {t("marketIntelligence.trends.subtitle")}
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <SegmentedControl
            value={mode}
            options={["sector", "theme"]}
            labelPrefix="marketIntelligence.replay.mode"
            onChange={(value) => setMode(value as "sector" | "theme")}
          />
          {trendWindowOptions(windows).map((window) => (
            <Button
              key={window}
              size="sm"
              variant={selectedWindow === window ? "default" : "outline"}
              onClick={() => onWindowChange(window)}
            >
              {t(`marketIntelligence.trends.windows.${window}`)}
            </Button>
          ))}
        </div>
      </div>

      {!hasTrendData ? (
        <Card className="p-6">
          <EmptyPlaceholder
            icon={<Icons.BarChart className="h-10 w-10" />}
            title={t("marketIntelligence.trends.emptyTitle")}
            description={t("marketIntelligence.trends.emptyDesc")}
          />
        </Card>
      ) : (
        <Card className="overflow-hidden bg-white">
          <div className="grid min-h-[620px] xl:grid-cols-[48%_52%]">
            <div className="border-border border-b xl:border-b-0 xl:border-r">
              <div className="border-border border-b px-4 py-3">
                <div className="flex items-center gap-2">
                  <h3 className="text-sm font-medium">{t("marketIntelligence.replay.ranking")}</h3>
                  <MetricLabelWithInfo
                    label=""
                    infoText={t("marketIntelligence.replay.rankingInfo")}
                  />
                </div>
                <p className="text-muted-foreground mt-1 text-xs">
                  {t("marketIntelligence.replay.rankingDesc")}
                </p>
              </div>
              <MirrorRankingBoard
                inflows={displayInflows}
                outflows={displayOutflows}
                inflowTitle={
                  inflowValueKind === "money"
                    ? t("marketIntelligence.trends.topInflows")
                    : t("marketIntelligence.trends.topRisers")
                }
                outflowTitle={
                  outflowValueKind === "money"
                    ? t("marketIntelligence.trends.topOutflows")
                    : t("marketIntelligence.trends.topFallers")
                }
                inflowValueKind={inflowValueKind}
                outflowValueKind={outflowValueKind}
                limit={18}
              />
            </div>
            <div className="min-w-0">
              <div className="border-border flex items-center justify-between gap-3 border-b px-4 py-3">
                <div>
                  <div className="flex items-center gap-2">
                    <h3 className="text-sm font-medium">
                      {mode === "sector"
                        ? t("marketIntelligence.trends.sectorRotation")
                        : t("marketIntelligence.trends.themeRotation")}
                    </h3>
                    <MetricLabelWithInfo
                      label=""
                      infoText={t("marketIntelligence.replay.chartInfo")}
                    />
                  </div>
                  <p className="text-muted-foreground mt-1 text-xs">
                    {t("marketIntelligence.replay.chartDesc")}
                  </p>
                </div>
                <Badge variant="outline" className="text-[11px]">
                  {chartSeries.length} {t("marketIntelligence.replay.lines")}
                </Badge>
              </div>
              <MarketReplayChart series={chartSeries} colorByKey={colorByKey} />
            </div>
          </div>
        </Card>
      )}
    </section>
  );
}

type RankingValueKind = "money" | "percent";

function themeTrendAsFlow(trend?: ThemeTrend): FlowTrend | undefined {
  if (!trend) return undefined;
  return {
    series: trend.momentumSeries.length > 0 ? trend.momentumSeries : trend.flowScoreSeries,
    topInflows: trend.topThemes,
    topOutflows: trend.bottomThemes ?? [],
  };
}

function themeRankingAsFlow(trend?: ThemeTrend): FlowTrend | undefined {
  if (!trend) return undefined;
  return {
    series: trend.flowScoreSeries,
    topInflows: trend.topThemes,
    topOutflows: trend.bottomThemes ?? [],
  };
}

function normalizeReplayTrend(trend?: FlowTrend): FlowTrend | undefined {
  if (!trend) return undefined;
  const series = trend.series
    .map(normalizePercentSeries)
    .sort((a, b) => Math.abs(b.cumulativeValue) - Math.abs(a.cumulativeValue));
  return {
    series,
    topInflows: replayRankings(series, true),
    topOutflows: replayRankings(series, false),
  };
}

function normalizePercentSeries(series: NamedTimeSeries): NamedTimeSeries {
  let compounded = 1;
  const points = series.points.map((point) => {
    const boundedValue = clampDailyPercent(point.value);
    compounded *= 1 + boundedValue / 100;
    return {
      date: point.date,
      value: boundedValue,
      cumulative: (compounded - 1) * 100,
    };
  });
  const latestPoint = points.at(-1);
  return {
    ...series,
    points,
    latestValue: latestPoint?.value ?? series.latestValue,
    cumulativeValue: latestPoint?.cumulative ?? 0,
  };
}

function replayRankings(series: NamedTimeSeries[], descending: boolean): RankingItem[] {
  return [...series]
    .sort((a, b) =>
      descending ? b.cumulativeValue - a.cumulativeValue : a.cumulativeValue - b.cumulativeValue,
    )
    .slice(0, 25)
    .map((item) => ({
      name: item.name,
      market: item.market,
      value: item.cumulativeValue,
      secondaryValue: item.latestValue,
      date: item.points.at(-1)?.date ?? null,
      source: item.source,
      ranking: null,
    }));
}

function clampDailyPercent(value: number): number {
  if (!Number.isFinite(value)) return 0;
  return Math.max(-30, Math.min(30, value));
}

function MirrorRankingBoard({
  inflows,
  outflows,
  inflowTitle,
  outflowTitle,
  inflowValueKind,
  outflowValueKind,
  limit,
}: {
  inflows: RankingItem[];
  outflows: RankingItem[];
  inflowTitle: string;
  outflowTitle: string;
  inflowValueKind: RankingValueKind;
  outflowValueKind: RankingValueKind;
  limit: number;
}) {
  const { t } = useTranslation();
  const leftItems = inflows.filter((item) => item.value > 0).slice(0, limit);
  const leftKeys = new Set(leftItems.map(rankingKey));
  const rightItems = outflows
    .filter((item) => item.value < 0 && !leftKeys.has(rankingKey(item)))
    .slice(0, limit);
  const max = Math.max(
    ...leftItems.map((item) => Math.abs(item.value)),
    ...rightItems.map((item) => Math.abs(item.value)),
    1,
  );

  return (
    <div className="p-4">
      <div className="grid grid-cols-2 gap-7">
        <p className="text-center text-xs font-medium text-muted-foreground">{inflowTitle}</p>
        <p className="text-center text-xs font-medium text-muted-foreground">{outflowTitle}</p>
      </div>
      <div className="relative mt-4 grid grid-cols-2 gap-7">
        <div className="absolute left-1/2 top-0 h-full w-px -translate-x-1/2 bg-border" />
        <div className="space-y-3">
          {leftItems.length === 0 ? (
            <p className="py-8 text-center text-xs text-muted-foreground">
              {t("marketIntelligence.noSnapshots")}
            </p>
          ) : (
            leftItems.map((item) => (
              <MirrorRankingRow
                key={`${item.market ?? "global"}-${item.name}`}
                item={item}
                max={max}
                side="left"
                valueKind={inflowValueKind}
              />
            ))
          )}
        </div>
        <div className="space-y-3">
          {rightItems.length === 0 ? (
            <p className="py-8 text-center text-xs text-muted-foreground">
              {t("marketIntelligence.noSnapshots")}
            </p>
          ) : (
            rightItems.map((item) => (
              <MirrorRankingRow
                key={`${item.market ?? "global"}-${item.name}`}
                item={item}
                max={max}
                side="right"
                valueKind={outflowValueKind}
              />
            ))
          )}
        </div>
      </div>
    </div>
  );
}

function rankingKey(item: RankingItem): string {
  return `${item.market ?? "global"}:${item.name}`;
}

function MirrorRankingRow({
  item,
  max,
  side,
  valueKind,
}: {
  item: RankingItem;
  max: number;
  side: "left" | "right";
  valueKind: RankingValueKind;
}) {
  const width = `${Math.max(3, (Math.abs(item.value) / max) * 100)}%`;
  const color = side === "left" ? "#d83a34" : "#167a44";
  const value =
    valueKind === "money" ? formatFlowAmount(item.value) : formatSignedPercent(item.value);

  if (side === "left") {
    return (
      <div className="grid grid-cols-[minmax(0,1fr)_118px] items-center gap-2 text-[11px]">
        <div className="min-w-0 text-right">
          <p className="truncate font-medium">{item.name}</p>
          <p className="truncate text-muted-foreground">{value}</p>
        </div>
        <div className="flex h-3 items-center justify-end">
          <div className="h-2.5 rounded-l-sm" style={{ width, backgroundColor: color }} />
        </div>
      </div>
    );
  }

  return (
    <div className="grid grid-cols-[118px_minmax(0,1fr)] items-center gap-2 text-[11px]">
      <div className="flex h-3 items-center justify-start">
        <div className="h-2.5 rounded-r-sm" style={{ width, backgroundColor: color }} />
      </div>
      <div className="min-w-0 text-left">
        <p className="truncate font-medium">{item.name}</p>
        <p className="truncate text-muted-foreground">{value}</p>
      </div>
    </div>
  );
}

function MarketReplayChart({
  series,
  colorByKey,
}: {
  series: NamedTimeSeries[];
  colorByKey: Map<string, string>;
}) {
  const { t } = useTranslation();
  const chartSeries = series.slice(0, 30);
  const chartData = buildLineChartData(chartSeries);
  const latestItems = chartSeries
    .map((item) => ({
      item,
      key: seriesKey(item),
      latest: item.points.at(-1)?.cumulative ?? item.cumulativeValue,
    }))
    .sort((a, b) => b.latest - a.latest);

  if (chartSeries.length === 0 || chartData.length < 2) {
    return (
      <div className="flex h-[560px] items-center justify-center px-6 text-center">
        <p className="text-muted-foreground text-sm">
          {t("marketIntelligence.trends.needMoreData")}
        </p>
      </div>
    );
  }

  return (
    <div className="grid h-[560px] min-w-0 grid-cols-[minmax(0,1fr)_180px]">
      <div className="min-w-0 px-2 py-4">
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={chartData} margin={{ top: 10, right: 12, left: 0, bottom: 4 }}>
            <CartesianGrid strokeDasharray="3 3" vertical={false} stroke="hsl(var(--border))" />
            <XAxis dataKey="date" tick={{ fontSize: 11 }} tickLine={false} axisLine={false} />
            <YAxis
              width={54}
              tick={{ fontSize: 11 }}
              tickFormatter={(value) => formatCompact(Number(value))}
              tickLine={false}
              axisLine={false}
            />
            <Tooltip content={<ReplayTooltip />} />
            {chartSeries.map((item, index) => {
              const key = seriesKey(item);
              return (
                <Line
                  key={key}
                  type="monotone"
                  dataKey={key}
                  stroke={colorByKey.get(key) ?? rankingBarColor("mixed", index)}
                  dot={false}
                  strokeWidth={index < 10 ? 2 : 1.2}
                  strokeOpacity={index < 12 ? 0.95 : 0.55}
                  isAnimationActive={false}
                  connectNulls
                />
              );
            })}
          </LineChart>
        </ResponsiveContainer>
      </div>
      <div className="border-border min-w-0 overflow-y-auto border-l px-3 py-4">
        <div className="space-y-1.5">
          {latestItems.map(({ key, latest }) => {
            const color = colorByKey.get(key) ?? rankingBarColor("mixed", 0);
            return (
              <div
                key={key}
                className="flex items-center justify-between gap-2 rounded border px-2 py-1 text-[11px]"
                style={{ borderColor: `${color}66` }}
              >
                <span className="min-w-0 truncate" style={{ color }}>
                  {key}
                </span>
                <span className="shrink-0 font-mono">{formatSignedPercent(latest)}</span>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}

function ReplayTooltip({ active, payload, label }: any) {
  if (!active || !payload || payload.length === 0) return null;
  const sorted = [...payload]
    .filter((entry: any) => entry.value != null)
    .sort((a: any, b: any) => b.value - a.value);
  return (
    <div className="bg-background border-border rounded-md border px-3 py-2 text-xs shadow-lg">
      <p className="text-muted-foreground mb-1.5 font-medium">{formatDate(String(label))}</p>
      <div className="space-y-0.5">
        {sorted.map((entry: any) => (
          <div key={entry.name} className="flex items-center gap-2">
            <span
              className="h-2 w-2 shrink-0 rounded-full"
              style={{ backgroundColor: entry.color }}
            />
            <span className="min-w-0 truncate">{entry.name}</span>
            <span className="shrink-0 font-mono">
              {formatSignedPercent(Number(entry.value))}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

type BreadthKind = "all" | "sector" | "theme";
type BreadthDirection = "all" | "up" | "down";

interface MarketBreadthItem {
  id: string;
  kind: "sector" | "theme";
  name: string;
  date: string;
  value: number | null;
  momentum: number | null;
  turnover: number | null;
  ranking: number | null;
  source: string | null;
}

function MarketBreadthPanel({
  sectors,
  themes,
  days,
}: {
  sectors: SectorRotationSnapshot[];
  themes: ThemeRotationSnapshot[];
  days: number | null;
}) {
  const { t } = useTranslation();
  const [kind, setKind] = useState<BreadthKind>("all");
  const [direction, setDirection] = useState<BreadthDirection>("all");
  const [query, setQuery] = useState("");
  const items = useMemo(() => buildBreadthItems(sectors, themes, days), [days, sectors, themes]);
  const filteredItems = useMemo(
    () => filterBreadthItems(items, { kind, direction, query }),
    [direction, items, kind, query],
  );
  const stats = useMemo(() => buildBreadthStats(filteredItems), [filteredItems]);

  return (
    <Card className="overflow-hidden">
      <div className="border-border flex flex-col gap-3 border-b px-4 py-3 xl:flex-row xl:items-center xl:justify-between">
        <div>
          <div className="flex items-center gap-2">
            <h2 className="text-sm font-medium">{t("marketIntelligence.breadth.title")}</h2>
            <MetricLabelWithInfo label="" infoText={t("marketIntelligence.breadth.info")} />
          </div>
          <p className="text-muted-foreground mt-1 text-xs">
            {t("marketIntelligence.breadth.subtitle")}
          </p>
        </div>
        <div className="flex flex-col gap-2 md:flex-row md:items-center">
          <input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder={t("marketIntelligence.breadth.search")}
            className="bg-background border-border h-9 min-w-52 rounded-md border px-3 text-sm"
          />
          <SegmentedControl
            value={kind}
            options={["all", "sector", "theme"]}
            labelPrefix="marketIntelligence.breadth.kind"
            onChange={(value) => setKind(value as BreadthKind)}
          />
          <SegmentedControl
            value={direction}
            options={["all", "up", "down"]}
            labelPrefix="marketIntelligence.breadth.direction"
            onChange={(value) => setDirection(value as BreadthDirection)}
          />
        </div>
      </div>

      <div className="grid border-b border-border md:grid-cols-4">
        <BreadthStat
          label={t("marketIntelligence.breadth.stats.total")}
          value={String(stats.total)}
        />
        <BreadthStat
          label={t("marketIntelligence.breadth.stats.up")}
          value={`${stats.up}`}
          tone="positive"
        />
        <BreadthStat
          label={t("marketIntelligence.breadth.stats.down")}
          value={`${stats.down}`}
          tone="negative"
        />
        <BreadthStat
          label={t("marketIntelligence.breadth.stats.average")}
          value={formatSignedPercent(stats.average)}
          tone={stats.average >= 0 ? "positive" : "negative"}
        />
      </div>

      {filteredItems.length === 0 ? (
        <div className="flex h-40 items-center justify-center px-6 text-center">
          <p className="text-muted-foreground text-sm">{t("marketIntelligence.noSnapshots")}</p>
        </div>
      ) : (
        <div className="grid max-h-[560px] gap-2 overflow-y-auto p-3 sm:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-4">
          {filteredItems.map((item) => (
            <BreadthTile key={item.id} item={item} />
          ))}
        </div>
      )}
    </Card>
  );
}

function SegmentedControl({
  value,
  options,
  labelPrefix,
  onChange,
}: {
  value: string;
  options: string[];
  labelPrefix: string;
  onChange: (value: string) => void;
}) {
  const { t } = useTranslation();
  return (
    <div className="bg-muted/40 border-border flex h-9 rounded-md border p-0.5">
      {options.map((option) => (
        <button
          key={option}
          type="button"
          onClick={() => onChange(option)}
          className={`rounded px-3 text-xs transition ${
            value === option
              ? "bg-background text-foreground shadow-sm"
              : "text-muted-foreground hover:text-foreground"
          }`}
        >
          {t(`${labelPrefix}.${option}`)}
        </button>
      ))}
    </div>
  );
}

function BreadthStat({
  label,
  value,
  tone = "neutral",
}: {
  label: string;
  value: string;
  tone?: "neutral" | "positive" | "negative";
}) {
  const toneClass =
    tone === "positive" ? "text-success" : tone === "negative" ? "text-destructive" : "";
  return (
    <div className="border-border border-b px-4 py-3 md:border-b-0 md:border-r last:border-r-0">
      <p className="text-muted-foreground text-xs">{label}</p>
      <p className={`mt-1 font-mono text-lg font-semibold ${toneClass}`}>{value}</p>
    </div>
  );
}

function BreadthTile({ item }: { item: MarketBreadthItem }) {
  const { t } = useTranslation();
  const value = item.value ?? item.momentum ?? 0;
  const isPositive = value >= 0;
  const intensity = Math.min(1, Math.abs(value) / 8);
  const background = isPositive
    ? `rgba(191, 73, 65, ${0.08 + intensity * 0.22})`
    : `rgba(50, 132, 105, ${0.08 + intensity * 0.22})`;
  const border = isPositive ? "rgba(191, 73, 65, 0.25)" : "rgba(50, 132, 105, 0.25)";

  return (
    <div
      className="min-h-24 rounded-md border p-3"
      style={{ backgroundColor: background, borderColor: border }}
    >
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <p className="truncate text-sm font-medium">{item.name}</p>
          <p className="text-muted-foreground mt-1 text-[11px]">
            {item.kind === "sector" ? "Sector" : "Theme"} · {item.date}
            {item.ranking ? ` · #${item.ranking}` : ""}
          </p>
        </div>
        <Badge variant="outline" className="shrink-0 text-[10px]">
          {t(`marketIntelligence.breadth.kind.${item.kind}`)}
        </Badge>
      </div>
      <div className="mt-3 grid grid-cols-2 gap-2 text-xs">
        <div>
          <p className="text-muted-foreground">{t("marketIntelligence.breadth.metrics.window")}</p>
          <p className={isPositive ? "text-success font-mono" : "text-destructive font-mono"}>
            {formatSignedPercent(value)}
          </p>
        </div>
        <div>
          <p className="text-muted-foreground">{t("marketIntelligence.breadth.metrics.latest")}</p>
          <p className="font-mono">
            {item.momentum === null ? "-" : formatSignedPercent(item.momentum)}
          </p>
        </div>
      </div>
    </div>
  );
}

function hasTrendWindows(windows?: MarketIntelligenceTrendWindow[]): boolean {
  return (windows ?? []).some(
    (window) =>
      window.capitalFlow.series.length > 0 ||
      window.sectorRotation.series.length > 0 ||
      window.themeRotation.flowScoreSeries.length > 0,
  );
}

function buildBreadthItems(
  sectors: SectorRotationSnapshot[],
  themes: ThemeRotationSnapshot[],
  days: number | null,
): MarketBreadthItem[] {
  const visibleSectors = filterSnapshotsByRecentDates(sectors, days, (item) => item.date);
  const visibleThemes = filterSnapshotsByRecentDates(themes, days, (item) => item.date);
  return [
    ...aggregateSectorBreadthItems(visibleSectors),
    ...aggregateThemeBreadthItems(visibleThemes),
  ].sort((a, b) => Math.abs(b.value ?? 0) - Math.abs(a.value ?? 0));
}

function filterBreadthItems(
  items: MarketBreadthItem[],
  filters: { kind: BreadthKind; direction: BreadthDirection; query: string },
): MarketBreadthItem[] {
  const query = filters.query.trim().toLowerCase();
  return items.filter((item) => {
    const value = item.value ?? item.momentum ?? 0;
    if (filters.kind !== "all" && item.kind !== filters.kind) return false;
    if (filters.direction === "up" && value <= 0) return false;
    if (filters.direction === "down" && value >= 0) return false;
    return !query || item.name.toLowerCase().includes(query);
  });
}

function buildBreadthStats(items: MarketBreadthItem[]) {
  const values = items
    .map((item) => item.value)
    .filter((value): value is number => value !== null && Number.isFinite(value));
  const total = values.length;
  const up = values.filter((value) => value > 0).length;
  const down = values.filter((value) => value < 0).length;
  const average = total === 0 ? 0 : values.reduce((sum, value) => sum + value, 0) / total;
  return { total: items.length, up, down, average };
}

function aggregateSectorBreadthItems(sectors: SectorRotationSnapshot[]): MarketBreadthItem[] {
  const groups = groupBy(sectors, (item) => `${item.market}:${item.sector}`);
  return Array.from(groups.values()).flatMap((items) => {
    const sorted = [...items].sort((a, b) => a.date.localeCompare(b.date));
    const latest = sorted[sorted.length - 1];
    const value = compoundPercentValues(sorted.map((item) => item.changePct ?? null));
    if (!latest || value === null) return [];
    return [
      {
        id: `sector:${latest.market}:${latest.sector}`,
        kind: "sector" as const,
        name: latest.sector,
        date: latest.date,
        value,
        momentum: latest.changePct ?? null,
        turnover: latest.turnover ?? null,
        ranking: latest.ranking ?? null,
        source: latest.source,
      },
    ];
  });
}

function aggregateThemeBreadthItems(themes: ThemeRotationSnapshot[]): MarketBreadthItem[] {
  const groups = groupBy(themes, (item) => item.theme);
  return Array.from(groups.values()).flatMap((items) => {
    const sorted = [...items].sort((a, b) => a.date.localeCompare(b.date));
    const latest = sorted[sorted.length - 1];
    const value = compoundPercentValues(sorted.map((item) => item.momentum ?? null));
    if (!latest || value === null) return [];
    return [
      {
        id: `theme:${latest.theme}`,
        kind: "theme" as const,
        name: latest.theme,
        date: latest.date,
        value,
        momentum: latest.momentum ?? null,
        turnover: null,
        ranking: latest.ranking ?? null,
        source: "sector_rotation_rules",
      },
    ];
  });
}

function groupBy<T>(items: T[], keyOf: (item: T) => string): Map<string, T[]> {
  const groups = new Map<string, T[]>();
  for (const item of items) {
    const key = keyOf(item);
    groups.set(key, [...(groups.get(key) ?? []), item]);
  }
  return groups;
}

function compoundPercentValues(values: Array<number | null>): number | null {
  const numericValues = values.filter((value): value is number => value !== null);
  if (numericValues.length === 0) return null;
  const compounded = numericValues.reduce(
    (product, value) => product * (1 + clampDailyPercent(value) / 100),
    1,
  );
  return (compounded - 1) * 100;
}

function buildFallbackTrends(
  capitalFlow: CapitalFlowSnapshot[],
  sectorRotation: SectorRotationSnapshot[],
  themeRotation: ThemeRotationSnapshot[],
) {
  return {
    granularity: "daily",
    windows: ["3d", "5d", "10d", "20d", "history"].map((window) => {
      const days = window === "history" ? null : Number(window.replace("d", ""));
      return {
        window,
        days,
        capitalFlow: buildFallbackFlowTrend(
          filterSnapshotsByRecentDates(capitalFlow, days, (item) => item.date),
          (item) => item.date,
          (item) => item.category,
          (item) => item.market,
          (item) => item.netFlow,
          (item) => item.source,
        ),
        sectorRotation: buildFallbackFlowTrend(
          filterSnapshotsByRecentDates(sectorRotation, days, (item) => item.date),
          (item) => item.date,
          (item) => item.sector,
          (item) => item.market,
          (item) => item.netFlow ?? null,
          (item) => item.source,
          (item) => item.ranking ?? null,
        ),
        themeRotation: {
          flowScoreSeries: buildFallbackSeries(
            filterSnapshotsByRecentDates(themeRotation, days, (item) => item.date),
            (item) => item.date,
            (item) => item.theme,
            () => null,
            (item) => item.flowScore ?? null,
            () => "sector_rotation_rules",
            (item) => item.ranking ?? null,
          ),
          momentumSeries: buildFallbackSeries(
            filterSnapshotsByRecentDates(themeRotation, days, (item) => item.date),
            (item) => item.date,
            (item) => item.theme,
            () => null,
            (item) => item.momentum ?? null,
            () => "sector_rotation_rules",
            (item) => item.ranking ?? null,
          ),
          topThemes: buildFallbackRankings(
            buildFallbackSeries(
              filterSnapshotsByRecentDates(themeRotation, days, (item) => item.date),
              (item) => item.date,
              (item) => item.theme,
              () => null,
              (item) => item.flowScore ?? null,
              () => "sector_rotation_rules",
              (item) => item.ranking ?? null,
            ),
            true,
          ),
          bottomThemes: buildFallbackRankings(
            buildFallbackSeries(
              filterSnapshotsByRecentDates(themeRotation, days, (item) => item.date),
              (item) => item.date,
              (item) => item.theme,
              () => null,
              (item) => item.flowScore ?? null,
              () => "sector_rotation_rules",
              (item) => item.ranking ?? null,
            ),
            false,
          ),
        },
      };
    }),
  };
}

function buildFallbackFlowTrend<T>(
  snapshots: T[],
  dateOf: (item: T) => string,
  nameOf: (item: T) => string,
  marketOf: (item: T) => string | null,
  valueOf: (item: T) => number | null,
  sourceOf: (item: T) => string | null,
  rankingOf: (item: T) => number | null = () => null,
) {
  const series = buildFallbackSeries(
    snapshots,
    dateOf,
    nameOf,
    marketOf,
    valueOf,
    sourceOf,
    rankingOf,
  );
  return {
    series,
    topInflows: buildFallbackRankings(series, true),
    topOutflows: buildFallbackRankings(series, false),
  };
}

function buildFallbackSeries<T>(
  snapshots: T[],
  dateOf: (item: T) => string,
  nameOf: (item: T) => string,
  marketOf: (item: T) => string | null,
  valueOf: (item: T) => number | null,
  sourceOf: (item: T) => string | null,
  rankingOf: (item: T) => number | null,
): NamedTimeSeries[] {
  const groups = new Map<string, T[]>();
  for (const snapshot of snapshots) {
    const value = valueOf(snapshot);
    if (value === null || !Number.isFinite(value)) continue;
    const key = `${marketOf(snapshot) ?? "global"}:${nameOf(snapshot)}`;
    groups.set(key, [...(groups.get(key) ?? []), snapshot]);
  }

  return Array.from(groups.values())
    .map((items) => {
      const sorted = [...items].sort((a, b) => dateOf(a).localeCompare(dateOf(b)));
      let cumulative = 0;
      const points = sorted.map((item) => {
        const value = valueOf(item) ?? 0;
        cumulative += value;
        return { date: dateOf(item), value, cumulative };
      });
      const latest = sorted[sorted.length - 1];
      return {
        name: nameOf(latest),
        market: marketOf(latest),
        points,
        latestValue: valueOf(latest),
        cumulativeValue: cumulative,
        source: sourceOf(latest),
        ranking: rankingOf(latest),
      };
    })
    .sort((a, b) => Math.abs(b.cumulativeValue) - Math.abs(a.cumulativeValue))
    .slice(0, 12);
}

function buildFallbackRankings(
  series: Array<NamedTimeSeries & { ranking?: number | null }>,
  descending: boolean,
): RankingItem[] {
  return [...series]
    .sort((a, b) =>
      descending ? b.cumulativeValue - a.cumulativeValue : a.cumulativeValue - b.cumulativeValue,
    )
    .slice(0, 10)
    .map((item) => ({
      name: item.name,
      market: item.market,
      value: item.cumulativeValue,
      secondaryValue: item.latestValue,
      ranking: item.ranking,
      date: item.points.at(-1)?.date ?? null,
      source: item.source,
    }));
}

function filterSnapshotsByRecentDates<T>(
  snapshots: T[],
  days: number | null,
  dateOf: (item: T) => string,
): T[] {
  if (!days) return snapshots;
  const dates = Array.from(new Set(snapshots.map(dateOf))).sort().slice(-days);
  return snapshots.filter((item) => dates.includes(dateOf(item)));
}

function trendWindowOptions(windows: MarketIntelligenceTrendWindow[]): string[] {
  const available = windows.map((window) => window.window);
  const preferred = ["3d", "5d", "10d", "20d", "history"];
  return preferred.filter((window) => available.includes(window));
}

function buildLineChartData(series: NamedTimeSeries[]): Array<Record<string, string | number | null>> {
  const rows = new Map<string, Record<string, string | number | null>>();
  for (const item of series) {
    const key = seriesKey(item);
    for (const point of item.points) {
      const row = rows.get(point.date) ?? { date: point.date };
      row[key] = point.cumulative;
      rows.set(point.date, row);
    }
  }
  return Array.from(rows.entries())
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([, row]) => row);
}

function seriesKey(series: NamedTimeSeries): string {
  return series.market ? `${series.name} · ${series.market}` : series.name;
}

function buildSeriesColorMap(
  series: NamedTimeSeries[],
  palette: "warm" | "cool" | "mixed",
  limit = 12,
): Map<string, string> {
  return new Map(
    series.slice(0, limit).map((item, index) => [seriesKey(item), rankingBarColor(palette, index)]),
  );
}

const rankingBarPalettes = {
  warm: ["#d98d88", "#d6a16d", "#cfb66f", "#c99aa2", "#c9867a", "#d0a083"],
  cool: ["#83b6a4", "#7fb8b4", "#82aecd", "#8e9fd0", "#9c96c9", "#7daec1"],
  mixed: ["#d98d88", "#d6a16d", "#91b98a", "#7fb8b4", "#82aecd", "#aa96cf"],
};

function rankingBarColor(palette: "warm" | "cool" | "mixed", index: number): string {
  const colors = rankingBarPalettes[palette];
  return colors[index % colors.length];
}

interface MergedExposure {
  theme: string;
  weightPct: number;
  marketValue: number;
  source: string;
  timestamp: string;
  momentum: number | null;
  assessment: "hot_heavy" | "warm" | "cold_heavy" | "neutral";
}

function PortfolioExposureCard({
  items,
  status,
}: {
  items: MergedExposure[];
  status?: { available: boolean; message?: string | null; source?: string | null };
}) {
  const { t } = useTranslation();

  if (items.length === 0) {
    return (
      <Card className="overflow-hidden">
        <div className="border-border flex items-center gap-2 border-b px-4 py-3">
          <span className="text-muted-foreground">
            <Icons.PieChart className="h-5 w-5" />
          </span>
          <h2 className="text-sm font-medium">
            {t("marketIntelligence.sections.portfolioExposure")}
          </h2>
          <MetricLabelWithInfo
            label=""
            infoText={t("marketIntelligence.sectionsInfo.portfolioExposure")}
          />
        </div>
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
      </Card>
    );
  }

  const assessments: Record<string, { emoji: string; label: string; detail: string; tone: string }> = {
    hot_heavy: { emoji: "🔥", label: "重仓强势", detail: "重仓顺势，动量强劲", tone: "text-red-600" },
    warm: { emoji: "✅", label: "顺势", detail: "顺势，仓位可考虑增加", tone: "text-green-600" },
    cold_heavy: { emoji: "⚠️", label: "重仓逆势", detail: "重仓逆势，注意风险", tone: "text-amber-600" },
    neutral: { emoji: "—", label: "低仓", detail: "低仓，影响有限", tone: "text-muted-foreground" },
  };

  const headerClass = "text-muted-foreground text-[11px] font-medium";

  return (
    <Card className="max-w-2xl overflow-hidden">
      <div className="border-border flex items-center gap-2 border-b px-4 py-3">
        <span className="text-muted-foreground">
          <Icons.PieChart className="h-5 w-5" />
        </span>
        <h2 className="text-sm font-medium">
          组合主题暴露 vs 市场趋势
        </h2>
        <MetricLabelWithInfo
          label=""
          infoText={t("marketIntelligence.sectionsInfo.portfolioExposure")}
        />
      </div>

      <div className="border-border flex items-center justify-between border-b px-4 py-2">
        <p className="text-muted-foreground text-xs">
          基于基金穿透分析
          {" · "}
          {items.length} 个主题
          <span className="ml-1 text-[10px]">（主题间存在重叠分类）</span>
        </p>
        <div className="flex gap-x-3 text-[11px]">
          {Object.entries(assessments).map(([, { emoji, label, tone }]) => (
            <span key={label} className={`flex items-center gap-1 ${tone}`}>
              {emoji} {label}
            </span>
          ))}
        </div>
      </div>

      <div className="border-border border-b px-4 py-2">
        <div className="grid grid-cols-[2fr_1fr_2fr_2fr] gap-4 text-[11px]">
          <span className={headerClass}>主题</span>
          <span className={`${headerClass} text-right`}>仓位</span>
          <span className={`${headerClass} text-right`}>5日动量</span>
          <span className={`${headerClass} pl-2`}>暴露评估</span>
        </div>
      </div>

      <div className="divide-border divide-y">
        {items.map((item) => {
          const momentumSign = (item.momentum ?? 0) >= 0 ? "+" : "";
          const momentumArrows =
            item.momentum !== null
              ? item.momentum > 2
                ? "↑↑"
                : item.momentum > 0
                  ? "↑"
                  : item.momentum < -2
                    ? "↓↓"
                    : "↓"
              : "→";
          const ass = assessments[item.assessment];

          return (
            <div
              key={item.theme}
              className="grid grid-cols-[2fr_1fr_2fr_2fr] items-center gap-4 px-4 py-2.5 hover:bg-muted/30"
            >
              <span className="truncate text-sm font-medium">{item.theme}</span>
              <span className="text-right font-mono text-xs">
                {item.weightPct.toFixed(1)}%
              </span>
              <span
                className={`text-right font-mono text-xs ${
                  item.momentum !== null
                    ? item.momentum >= 0
                      ? "text-red-600"
                      : "text-green-600"
                    : "text-muted-foreground"
                }`}
              >
                {item.momentum !== null
                  ? `${momentumSign}${item.momentum.toFixed(1)}% ${momentumArrows}`
                  : "--"}
              </span>
              <span className={`truncate pl-2 text-xs ${ass.tone}`}>
                {ass.emoji} {ass.detail}
              </span>
            </div>
          );
        })}
      </div>
    </Card>
  );
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

function formatFlowAmount(value: number): string {
  const absValue = Math.abs(value);
  if (absValue >= 100_000_000) {
    return `${value >= 0 ? "+" : "-"}${(absValue / 100_000_000).toFixed(2)}亿`;
  }
  if (absValue >= 10_000) {
    return `${value >= 0 ? "+" : "-"}${(absValue / 10_000).toFixed(2)}万`;
  }
  return `${value >= 0 ? "+" : "-"}${absValue.toFixed(2)}`;
}

function formatCompact(value: number): string {
  return new Intl.NumberFormat(undefined, {
    notation: "compact",
    maximumFractionDigits: 1,
  }).format(value);
}

function formatSignedPercent(value: number): string {
  const sign = value > 0 ? "+" : "";
  return `${sign}${value.toFixed(2)}%`;
}

function formatDate(value: string): string {
  const date = new Date(`${value}T00:00:00`);
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
  }).format(date);
}
