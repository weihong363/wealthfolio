import type {
  Exchange,
  StockBoardInfo,
  StockConceptInfo,
  StockIndustryInfo,
  StockPricePoint,
  StockThemeMetrics,
  StockThemeProfile,
} from "./types";
import { cleanAssetName } from "@/pages/fund-research/utils";

/** Raw Eastmoney stock quote response */
interface EmQuoteResponse {
  data?: {
    f43?: number;
    f47?: number;
    f48?: number;
    f50?: number;
    f57?: string;
    f58?: string;
    f116?: number;
    f127?: string;
    f128?: string;
    f129?: string;
    f169?: number;
    f170?: number;
  };
}

export function mapStockQuote(
  stockCode: string,
  exchange: Exchange,
  secid: string,
  raw: EmQuoteResponse,
): Partial<StockThemeProfile> {
  const d = raw.data;
  if (!d) return { stockCode, exchange, secid };

  return {
    stockCode,
    exchange,
    secid,
    stockName: cleanAssetName(d.f58 ?? ""),
    latestPrice: d.f43 !== undefined ? d.f43 / 100 : undefined,
    changePercent: d.f170 !== undefined ? d.f170 / 100 : undefined,
    turnover: d.f50,
    volume: d.f47,
    amount: d.f48,
    marketCap: d.f116,
  };
}

/**
 * Extract industry, concepts, and boards from the extended quote response
 * (f127=industry, f128=region, f129=concepts comma-separated).
 * Used for US/HK stocks where F10 CoreConception is not available.
 */
export function mapQuoteProfile(raw: EmQuoteResponse): {
  industry?: StockIndustryInfo;
  concepts: StockConceptInfo[];
  boards: StockBoardInfo[];
} {
  const d = raw.data;
  if (!d) return { concepts: [], boards: [] };

  const industryName = cleanOrDefault(d.f127);
  const regionName = cleanOrDefault(d.f128);
  const conceptNames = d.f129
    ? d.f129
        .split(",")
        .map((c) => cleanAssetName(c.trim()))
        .filter(Boolean)
    : [];

  const industry: StockIndustryInfo | undefined = industryName
    ? { boardName: industryName }
    : undefined;

  const concepts: StockConceptInfo[] = conceptNames.map((name) => ({ name }));

  const boards: StockBoardInfo[] = [];
  if (industryName) {
    boards.push({ name: industryName, type: "industry" });
  }
  if (regionName) {
    boards.push({ name: regionName, type: "region" });
  }
  for (const name of conceptNames) {
    boards.push({ name, type: "concept" });
  }

  return { industry, concepts, boards };
}

function cleanOrDefault(value: string | undefined | null): string | undefined {
  if (!value) return undefined;
  const cleaned = cleanAssetName(value.trim());
  if (!cleaned || cleaned === "-" || cleaned === "--") return undefined;
  return cleaned;
}

/** F10 CoreConception — ssbk (所属板块) row */
interface EmBoardItem {
  SECUCODE?: string;
  SECURITY_CODE?: string;
  BOARD_CODE?: string;
  BOARD_NAME?: string;
  IS_PRECISE?: unknown;
  BOARD_RANK?: number;
  BOARD_TYPE?: string;
}

/** F10 CoreConception — hxtc (核心题材) row */
interface EmConceptItem {
  SECUCODE?: string;
  KEYWORD?: string;
  MAINPOINT?: number;
  MAINPOINT_CONTENT?: string;
  KEY_CLASSIF?: string;
}

/** F10 CoreConception full response */
interface EmF10CoreConception {
  ssbk?: EmBoardItem[];
  hxtc?: EmConceptItem[];
  zyzb?: unknown[];
}

/**
 * Parse boards (ssbk) from F10 CoreConception response.
 * ssbk contains industry + concept + region boards.
 */
export function mapF10Boards(raw: EmF10CoreConception | null): StockBoardInfo[] {
  const rows = raw?.ssbk;
  if (!rows?.length) return [];

  return rows.map((r) => {
    const name = cleanAssetName(r.BOARD_NAME ?? "");
    return {
      code: r.BOARD_CODE,
      name,
      type: classifyBoardType(name, r.BOARD_RANK),
    };
  });
}

/**
 * Parse core themes (hxtc) from F10 CoreConception response.
 * These are the stock's key conceptual/thematic classifications.
 */
export function mapF10Concepts(raw: EmF10CoreConception | null): StockConceptInfo[] {
  const rows = raw?.hxtc;
  if (!rows?.length) return [];

  return rows.map((r) => {
    const name = cleanAssetName(r.KEYWORD ?? "");
    return {
      name,
    };
  });
}

/**
 * Extract the primary industry from boards — usually BOARD_RANK=1 or similar.
 */
export function mapF10Industry(raw: EmF10CoreConception | null): StockIndustryInfo | undefined {
  const rows = raw?.ssbk;
  if (!rows?.length) return undefined;

  // Primary industry is typically the one with BOARD_RANK=1
  const primary = rows.find((r) => r.BOARD_RANK === 1) ?? rows[0];
  return {
    boardCode: primary.BOARD_CODE,
    boardName: cleanAssetName(primary.BOARD_NAME ?? ""),
    rankInBoard: primary.BOARD_RANK,
  };
}

function classifyBoardType(name: string, rank?: number): StockBoardInfo["type"] {
  // Industry boards typically have rank=1
  if (rank === 1) return "industry";
  // Common concept keywords
  if (name.includes("概念") || name.includes("题材") || name.includes("主题")) return "concept";
  if (name.includes("地区") || name.includes("地域")) return "region";
  return "unknown";
}

/** Raw Eastmoney board quote (from clist API) */
interface EmBoardQuoteItem {
  f2?: number;
  f3?: number;
  f12?: string;
  f14?: string;
  f62?: number;
  f66?: number;
}

interface EmBoardQuoteResponse {
  data?: {
    diff?: EmBoardQuoteItem[];
  } | null;
}

export function mapBoardQuote(raw: EmBoardQuoteResponse | null): {
  changePercent?: number;
  mainNetInflow?: number;
  amount?: number;
} {
  const items = raw?.data?.diff;
  if (!items?.length) return {};
  const item = items[0];
  return {
    changePercent: item.f3,
    mainNetInflow: item.f62,
    amount: item.f66,
  };
}

interface EmKlineResponse {
  data?: {
    klines?: string[];
  } | null;
}

export function mapPriceHistory(raw: EmKlineResponse | null): StockPricePoint[] {
  const rows = raw?.data?.klines;
  if (!rows?.length) return [];

  return rows
    .map((row) => {
      const [date, open, close, high, low, volume, amount, , changePercent] = row.split(",");
      return {
        date,
        open: parseOptionalNumber(open),
        close: parseOptionalNumber(close),
        high: parseOptionalNumber(high),
        low: parseOptionalNumber(low),
        volume: parseOptionalNumber(volume),
        amount: parseOptionalNumber(amount),
        changePercent: parseOptionalNumber(changePercent),
      };
    })
    .filter((point) => point.date && point.close !== undefined);
}

function parseOptionalNumber(value: string | undefined): number | undefined {
  if (!value || value === "-" || value === "--") return undefined;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : undefined;
}

// ─── Metrics computation ─────────────────────────────────────────────────────

const HOT_CONCEPT_NAMES = new Set([
  "人工智能",
  "AI",
  "芯片",
  "半导体",
  "新能源",
  "光伏",
  "锂电池",
  "机器人",
  "低空经济",
  "量子",
  "算力",
  "大数据",
  "云计算",
  "自动驾驶",
  "智能汽车",
  "创新药",
  "减肥药",
  "数字经济",
  "军工",
  "信创",
  "数据要素",
  "消费电子",
  "华为",
  "鸿蒙",
  "5G",
  "光通信",
  "CPO",
]);

export function computeMetrics(
  concepts: StockConceptInfo[],
  industry?: StockIndustryInfo,
): StockThemeMetrics {
  const conceptCount = concepts.length;
  const hotConcepts = concepts.filter((c) => isHotConcept(c));
  const hotConceptCount = hotConcepts.length;

  const candidates = hotConcepts.length > 0 ? hotConcepts : concepts;
  const sortedByScore = [...candidates].sort((a, b) => {
    const aScore = (a.changePercent ?? 0) + (a.mainNetInflow ?? 0) / 1e8;
    const bScore = (b.changePercent ?? 0) + (b.mainNetInflow ?? 0) / 1e8;
    return bScore - aScore;
  });
  const dominantTheme = sortedByScore[0]?.name;

  const themeExposureScore = Math.min(100, conceptCount * 5 + hotConceptCount * 10);

  const topByHeat = [...concepts]
    .sort((a, b) => (b.heatRank ?? 1000) - (a.heatRank ?? 1000))
    .slice(0, 5)
    .map((c) => c.name);

  const topByInflow = [...concepts]
    .sort((a, b) => (b.mainNetInflow ?? 0) - (a.mainNetInflow ?? 0))
    .slice(0, 5)
    .map((c) => c.name);

  const fundRelevanceTags = [
    ...new Set(
      [industry?.boardName, ...concepts.map((c) => c.name), ...topByHeat].filter(
        Boolean,
      ) as string[],
    ),
  ].slice(0, 20);

  const avgChange =
    concepts.length > 0
      ? concepts.reduce((s, c) => s + (c.changePercent ?? 0), 0) / concepts.length
      : 0;

  return {
    conceptCount,
    hotConceptCount,
    dominantTheme,
    themeExposureScore,
    industryMomentumScore: industry?.changePercent,
    conceptMomentumScore: avgChange,
    topConceptsByHeat: topByHeat,
    topConceptsByMainInflow: topByInflow,
    fundRelevanceTags,
  };
}

function isHotConcept(c: StockConceptInfo): boolean {
  if (c.isHot) return true;
  const name = c.name;
  for (const hot of HOT_CONCEPT_NAMES) {
    if (name.includes(hot)) return true;
  }
  if ((c.heatRank ?? 999) <= 50) return true;
  return false;
}
