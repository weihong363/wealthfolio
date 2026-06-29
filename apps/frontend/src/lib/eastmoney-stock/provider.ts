import type {
  BatchStockProfileResult,
  FundThemeExposure,
  FundHoldingTheme,
  StockConceptInfo,
  StockPricePoint,
  StockThemeProfile,
} from "./types";
import { normalizeStockCode, buildEastMoneySecid } from "./code-utils";
import {
  boardPageUrl,
  boardQuoteUrl,
  stockF10CoreConceptionUrl,
  stockF10PageUrl,
  stockKlineUrl,
  stockQuoteUrl,
  stockQuoteExtendedUrl,
} from "./endpoints";
import { fetchJson, cacheInvalidate } from "./http-client";
import {
  computeMetrics,
  mapBoardQuote,
  mapF10Boards,
  mapF10Concepts,
  mapF10Industry,
  mapPriceHistory,
  mapQuoteProfile,
  mapStockQuote,
} from "./mapper";

// Cache TTLs
const QUOTE_TTL_MS = 3 * 60 * 1000;
const BOARD_TTL_MS = 15 * 60 * 1000;
const CLASSIFICATION_TTL_MS = 24 * 60 * 60 * 1000;
const HISTORY_TTL_MS = 30 * 60 * 1000;

// F10 CoreConception response type
interface EmF10CoreConception {
  ssbk?: Record<string, unknown>[];
  hxtc?: Record<string, unknown>[];
}

export class EastMoneyStockProvider {
  async getStockThemeProfile(rawCode: string): Promise<StockThemeProfile> {
    const { code, exchange } = normalizeStockCode(rawCode);
    const secid = buildEastMoneySecid(code, exchange);
    const sourceUrls: string[] = [];
    const fetchedAt = new Date().toISOString();

    // 1. Fetch real-time quote
    const quoteUrl = stockQuoteUrl(secid);
    sourceUrls.push(quoteUrl);
    const quoteRaw = await fetchJson<Record<string, unknown>>(
      quoteUrl,
      { referer: "https://quote.eastmoney.com/" },
      QUOTE_TTL_MS,
    );
    const quotePartial = mapStockQuote(
      code,
      exchange,
      secid,
      quoteRaw as Parameters<typeof mapStockQuote>[3],
    );

    // 2. Fetch industry / concepts — route based on exchange
    let industry: StockThemeProfile["industry"];
    let concepts: StockThemeProfile["concepts"];
    let boards: StockThemeProfile["boards"];

    if (exchange === "SH" || exchange === "SZ" || exchange === "BJ") {
      // A-share: rich data from F10 CoreConception
      const exchPrefix = exchange === "SH" ? "SH" : "SZ";
      const f10Url = stockF10CoreConceptionUrl(code, exchPrefix === "SH" ? "SH" : "SZ");
      sourceUrls.push(f10Url);
      const f10Referer = stockF10PageUrl(exchPrefix === "SH" ? "SH" : "SZ", code);
      try {
        const f10Raw = await fetchJson<EmF10CoreConception>(
          f10Url,
          { referer: f10Referer },
          CLASSIFICATION_TTL_MS,
        );
        boards = mapF10Boards(f10Raw);
        concepts = mapF10Concepts(f10Raw);
        industry = mapF10Industry(f10Raw);
      } catch {
        concepts = [];
        boards = [];
      }
    } else {
      // US / HK: basic data from extended quote (f127,f128,f129)
      const extUrl = stockQuoteExtendedUrl(secid);
      sourceUrls.push(extUrl);
      try {
        const extRaw = await fetchJson<Record<string, unknown>>(
          extUrl,
          { referer: "https://quote.eastmoney.com/" },
          CLASSIFICATION_TTL_MS,
        );
        const profile = mapQuoteProfile(extRaw as Parameters<typeof mapStockQuote>[3]);
        industry = profile.industry;
        concepts = profile.concepts;
        boards = profile.boards;
      } catch {
        // Non-critical — continue without industry/concept data
        concepts = [];
        boards = [];
      }
    }
    const allConceptBoards = [
      ...concepts,
      ...boards.filter((b) => b.type === "concept").map((b) => ({ name: b.name, code: b.code })),
    ];

    // 3. Enrich with real-time board data (concepts only — limit to 10 for speed)
    await this.enrichConceptBoards(allConceptBoards.slice(0, 10), sourceUrls);

    // 4. Compute derived metrics
    const metrics = computeMetrics(concepts, industry);

    return {
      stockCode: code,
      stockName: quotePartial.stockName,
      exchange,
      secid,
      latestPrice: quotePartial.latestPrice,
      changePercent: quotePartial.changePercent,
      turnover: quotePartial.turnover,
      volume: quotePartial.volume,
      amount: quotePartial.amount,
      industry,
      concepts,
      boards,
      metrics,
      source: "eastmoney",
      sourceUrls,
      fetchedAt,
    };
  }

  async getStockThemeProfiles(rawCodes: string[]): Promise<BatchStockProfileResult> {
    const results = await Promise.allSettled(
      rawCodes.map((code) => this.getStockThemeProfile(code)),
    );

    const profiles: StockThemeProfile[] = [];
    const errors: { stockCode: string; message: string }[] = [];

    for (let i = 0; i < results.length; i++) {
      const result = results[i];
      if (result.status === "fulfilled") {
        profiles.push(result.value);
      } else {
        errors.push({
          stockCode: rawCodes[i],
          message: result.reason instanceof Error ? result.reason.message : String(result.reason),
        });
      }
    }

    return { profiles, errors };
  }

  async getStockPriceHistory(rawCode: string, days = 120): Promise<StockPricePoint[]> {
    const { code, exchange } = normalizeStockCode(rawCode);
    const secid = buildEastMoneySecid(code, exchange);
    const beg = yyyymmdd(daysAgo(days * 2));
    const url = stockKlineUrl(secid, beg);
    const raw = await fetchJson<Record<string, unknown>>(
      url,
      { referer: "https://quote.eastmoney.com/" },
      HISTORY_TTL_MS,
    );
    return mapPriceHistory(raw).slice(-days);
  }

  async getFundThemeExposure(
    fundCode: string,
    fundName: string | undefined,
    holdings: { stockCode: string; stockName: string; weight?: number }[],
  ): Promise<FundThemeExposure> {
    const stockCodes = holdings.map((h) => h.stockCode);
    const { profiles } = await this.getStockThemeProfiles(stockCodes);

    const profileMap = new Map(profiles.map((p) => [p.stockCode, p]));

    const holdingThemes: FundHoldingTheme[] = [];
    const industryWeights = new Map<string, { weight: number; count: number }>();
    const conceptWeights = new Map<
      string,
      { weight: number; count: number; momentumSum: number }
    >();
    const themeSet = new Set<string>();
    let totalWeight = 0;

    for (const holding of holdings) {
      const { code } = normalizeStockCode(holding.stockCode);
      const profile = profileMap.get(code);
      const weight = holding.weight ?? 0;
      totalWeight += weight;

      const hTheme: FundHoldingTheme = {
        stockCode: code,
        stockName: profile?.stockName ?? holding.stockName,
        weight,
        industry: profile?.industry?.boardName,
        concepts: profile?.concepts.map((c) => c.name) ?? [],
        dominantTheme: profile?.metrics.dominantTheme,
        themeExposureScore: profile?.metrics.themeExposureScore,
      };
      holdingThemes.push(hTheme);

      if (profile?.industry?.boardName) {
        const existing = industryWeights.get(profile.industry.boardName) ?? {
          weight: 0,
          count: 0,
        };
        industryWeights.set(profile.industry.boardName, {
          weight: existing.weight + weight,
          count: existing.count + 1,
        });
      }

      for (const concept of profile?.concepts ?? []) {
        const existing = conceptWeights.get(concept.name) ?? {
          weight: 0,
          count: 0,
          momentumSum: 0,
        };
        conceptWeights.set(concept.name, {
          weight: existing.weight + weight,
          count: existing.count + 1,
          momentumSum: existing.momentumSum + (concept.changePercent ?? 0),
        });
      }

      if (profile?.metrics.dominantTheme) {
        themeSet.add(profile.metrics.dominantTheme);
      }
    }

    const industryDistribution = [...industryWeights.entries()]
      .map(([industry, { weight: w, count }]) => ({
        industry,
        weight: totalWeight > 0 ? w / totalWeight : 0,
        stockCount: count,
      }))
      .sort((a, b) => b.weight - a.weight);

    const conceptDistribution = [...conceptWeights.entries()]
      .map(([concept, { weight: w, count, momentumSum }]) => ({
        concept,
        weight: totalWeight > 0 ? w / totalWeight : 0,
        stockCount: count,
        avgMomentumScore: count > 0 ? momentumSum / count : undefined,
      }))
      .sort((a, b) => b.weight - a.weight);

    const hotConceptNames = conceptDistribution
      .filter((c) => c.avgMomentumScore !== undefined && c.avgMomentumScore > 0)
      .slice(0, 5);
    const hotThemeWeight = hotConceptNames.reduce((s, c) => s + c.weight, 0);

    return {
      fundCode,
      fundName,
      holdings: holdingThemes,
      industryDistribution,
      conceptDistribution,
      dominantThemes: [...themeSet].slice(0, 10),
      hotThemeWeight,
      fetchedAt: new Date().toISOString(),
    };
  }

  clearCache(pattern?: string): void {
    cacheInvalidate(pattern);
  }

  // ── Private helpers ────────────────────────────────────────────────────────

  private async enrichConceptBoards(
    concepts: { name: string; code?: string }[],
    _sourceUrls: string[],
  ): Promise<void> {
    const withCodes = concepts.filter((c) => c.code);
    const BATCH = 3;
    for (let i = 0; i < withCodes.length; i += BATCH) {
      const batch = withCodes.slice(i, i + BATCH);
      await Promise.allSettled(
        batch.map(async (c) => {
          try {
            const url = boardQuoteUrl(c.code!, 2);
            const raw = await fetchJson<Record<string, unknown>>(
              url,
              { referer: boardPageUrl(c.code!, "concept") },
              BOARD_TTL_MS,
            );
            const q = mapBoardQuote(raw as Parameters<typeof mapBoardQuote>[0]);
            // Mutate the concept in-place
            if (q.changePercent !== undefined) {
              (c as StockConceptInfo).changePercent = q.changePercent;
            }
            if (q.mainNetInflow !== undefined) {
              (c as StockConceptInfo).mainNetInflow = q.mainNetInflow;
            }
            if (q.amount !== undefined) {
              (c as StockConceptInfo).amount = q.amount;
            }
          } catch {
            // Non-critical
          }
        }),
      );
      if (i + BATCH < withCodes.length) {
        await new Promise((r) => setTimeout(r, 200));
      }
    }
  }
}

function daysAgo(days: number): Date {
  const date = new Date();
  date.setDate(date.getDate() - days);
  return date;
}

function yyyymmdd(date: Date): string {
  const year = date.getFullYear();
  const month = `${date.getMonth() + 1}`.padStart(2, "0");
  const day = `${date.getDate()}`.padStart(2, "0");
  return `${year}${month}${day}`;
}
