import { getMarketIntelligenceSummary } from "@/adapters";
import {
  fetchEastmoneyMarketIntelligenceSnapshots,
  fetchEastmoneyMarketIntelligenceSnapshotsForRange,
} from "../../../adapters/shared/eastmoney-market-intelligence";
import { ingestMarketIntelligenceSnapshots } from "../../../adapters/shared/market-intelligence";
import { useQuery } from "@tanstack/react-query";

async function fetchSummary(portfolioId?: string, window = "20d") {
  const backend = await getMarketIntelligenceSummary(portfolioId, window);
  const backfilled = await backfillMissingHistoricalSnapshots(backend, portfolioId, window);

  if (hasUsableTrendData(backfilled)) {
    return backfilled;
  }

  try {
    const snapshots = await fetchEastmoneyMarketIntelligenceSnapshots();
    if (
      snapshots.marketOverview.length > 0 ||
      snapshots.capitalFlow.length > 0 ||
      snapshots.sectorRotation.length > 0
    ) {
      await ingestMarketIntelligenceSnapshots({
        marketOverview:
          snapshots.marketOverview.length > 0 ? snapshots.marketOverview : backend.marketOverview,
        capitalFlow:
          snapshots.capitalFlow.length > 0 ? snapshots.capitalFlow : backend.capitalFlow,
        sectorRotation:
          snapshots.sectorRotation.length > 0
            ? snapshots.sectorRotation
            : backfilled.sectorRotation,
      });
      return await getMarketIntelligenceSummary(portfolioId, window);
    }
  } catch {
    // Browser fetch failed; return backend data as-is
  }

  return backfilled;
}

async function backfillMissingHistoricalSnapshots(
  summary: Awaited<ReturnType<typeof getMarketIntelligenceSummary>>,
  portfolioId?: string,
  window = "20d",
) {
  const missingDates = missingWeekdayDates(summaryDates(summary), window);
  if (missingDates.length === 0) return summary;

  try {
    const snapshots = await fetchEastmoneyMarketIntelligenceSnapshotsForRange(
      missingDates[0],
      missingDates[missingDates.length - 1],
    );
    if (snapshots.sectorRotation.length === 0) return summary;
    await ingestMarketIntelligenceSnapshots({
      marketOverview: summary.marketOverview,
      capitalFlow: summary.capitalFlow,
      sectorRotation: snapshots.sectorRotation.filter((snapshot) =>
        missingDates.includes(snapshot.date),
      ),
    });
    return await getMarketIntelligenceSummary(portfolioId, window);
  } catch {
    // Historical backfill is best-effort; keep rendering saved data.
    return summary;
  }
}

function hasUsableTrendData(summary: Awaited<ReturnType<typeof getMarketIntelligenceSummary>>) {
  const windows = summary.trends?.windows ?? [];
  return windows.some(
    (window) =>
      hasSeriesWithAtLeastTwoPoints(window.capitalFlow.series) ||
      hasSeriesWithAtLeastTwoPoints(window.sectorRotation.series) ||
      hasSeriesWithAtLeastTwoPoints(window.themeRotation.flowScoreSeries),
  );
}

function hasSeriesWithAtLeastTwoPoints(series: { points: unknown[] }[]) {
  return series.some((item) => item.points.length >= 2);
}

function summaryDates(summary: Awaited<ReturnType<typeof getMarketIntelligenceSummary>>): string[] {
  return [
    ...summary.capitalFlow.map((item) => item.date),
    ...summary.sectorRotation.map((item) => item.date),
    ...summary.themeRotation.map((item) => item.date),
    ...(summary.trends?.windows ?? []).flatMap((window) => [
      ...window.capitalFlow.series.flatMap((series) => series.points.map((point) => point.date)),
      ...window.sectorRotation.series.flatMap((series) => series.points.map((point) => point.date)),
      ...window.themeRotation.flowScoreSeries.flatMap((series) =>
        series.points.map((point) => point.date),
      ),
      ...window.themeRotation.momentumSeries.flatMap((series) =>
        series.points.map((point) => point.date),
      ),
    ]),
  ];
}

function missingWeekdayDates(dates: string[], window: string): string[] {
  const days = daysForWindow(window);
  if (!days) return [];
  const uniqueDates = Array.from(new Set(dates.filter(Boolean))).sort();
  const targetDates = targetWeekdayDates(latestWindowEndDate(uniqueDates), days);

  const existing = new Set(uniqueDates);
  return targetDates.filter((date) => !existing.has(date));
}

function targetWeekdayDates(endDate: string, days: number): string[] {
  const dates: string[] = [];
  const cursor = parseDateOnly(endDate);
  while (dates.length < days) {
    const day = cursor.getDay();
    if (day !== 0 && day !== 6) {
      dates.push(formatDateOnly(cursor));
    }
    cursor.setDate(cursor.getDate() - 1);
  }
  return dates.reverse();
}

function latestWindowEndDate(dates: string[]): string {
  const latestSavedDate = dates[dates.length - 1];
  const today = formatDateOnly(new Date());
  if (!latestSavedDate) return today;
  return latestSavedDate > today ? latestSavedDate : today;
}

function daysForWindow(window: string): number | null {
  const days = Number(window.replace("d", ""));
  return Number.isFinite(days) && days > 0 ? days : null;
}

function parseDateOnly(value: string): Date {
  return new Date(`${value}T00:00:00`);
}

function formatDateOnly(value: Date): string {
  const year = value.getFullYear();
  const month = String(value.getMonth() + 1).padStart(2, "0");
  const date = String(value.getDate()).padStart(2, "0");
  return `${year}-${month}-${date}`;
}

export function useMarketIntelligenceSummary(portfolioId?: string, window = "20d") {
  return useQuery({
    queryKey: ["marketIntelligenceSummary", portfolioId ?? "none", window],
    queryFn: () => fetchSummary(portfolioId, window),
    staleTime: Infinity,
    gcTime: 30 * 60_000,
    refetchOnMount: false,
    refetchOnReconnect: false,
    refetchOnWindowFocus: false,
  });
}
