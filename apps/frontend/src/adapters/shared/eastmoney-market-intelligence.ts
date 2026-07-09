import type {
  BrowserMarketIntelligenceSnapshots,
  CapitalFlowSnapshot,
  MarketSnapshot,
  SectorRotationSnapshot,
} from "@/pages/market-intelligence/types";

const PUSH2_BASE_URL = "https://push2.eastmoney.com";
const PUSH2HIS_BASE_URL = "https://push2his.eastmoney.com";
const EASTMONEY_UT = "bd1d9ddb04089700cf9c27f6f7426281";
const JSONP_TIMEOUT_MS = 15_000;
const SOURCE = "eastmoney_browser";
const HISTORICAL_BOARD_LIMIT = 300;

interface EastmoneyResponse {
  data?: {
    diff?: EastmoneyRow[];
  };
}

interface EastmoneyKlineResponse {
  data?: {
    klines?: string[];
  };
}

interface EastmoneyRow {
  f2?: number | string;
  f3?: number | string;
  f6?: number | string;
  f12?: string;
  f13?: number | string;
  f14?: string;
  f62?: number | string;
}

interface IndexDescriptor {
  market: string;
  name: string;
  code: string;
  secid: string;
}

const INDEX_DESCRIPTORS: IndexDescriptor[] = [
  { market: "CN", name: "上证指数", code: "000001", secid: "1.000001" },
  { market: "CN", name: "深证成指", code: "399001", secid: "0.399001" },
  { market: "CN", name: "创业板指", code: "399006", secid: "0.399006" },
  { market: "CN", name: "科创50", code: "000688", secid: "1.000688" },
  { market: "HK", name: "恒生指数", code: "HSI", secid: "100.HSI" },
  { market: "HK", name: "恒生科技", code: "HSTECH", secid: "100.HSTECH" },
];

export async function fetchEastmoneyMarketIntelligenceSnapshots(
  date?: string,
): Promise<BrowserMarketIntelligenceSnapshots> {
  if (date) {
    return fetchEastmoneyMarketIntelligenceSnapshotsForRange(date, date);
  }

  const [marketOverview, sectorRows] = await Promise.all([
    fetchMarketOverview().catch(() => []),
    fetchSectorRows().catch(() => []),
  ]);
  const sectorRotation = toSectorRotationSnapshots(sectorRows);
  return {
    marketOverview,
    capitalFlow: toCapitalFlowSnapshots(sectorRows),
    sectorRotation,
  };
}

export async function fetchEastmoneyMarketIntelligenceSnapshotsForRange(
  startDate: string,
  endDate: string,
): Promise<BrowserMarketIntelligenceSnapshots> {
  const sectorRows = await fetchSectorRows().catch(() => []);
  return {
    marketOverview: [],
    capitalFlow: [],
    sectorRotation: await fetchHistoricalSectorRotationRange(sectorRows, startDate, endDate),
  };
}

async function fetchMarketOverview(): Promise<MarketSnapshot[]> {
  const fields = "f12,f14,f2,f3,f4,f6";
  const secids = INDEX_DESCRIPTORS.map((item) => item.secid).join(",");
  const url = eastmoneyUrl(PUSH2_BASE_URL, "/api/qt/ulist.np/get", {
    fields,
    fltt: "2",
    secids,
  });
  const rows = await fetchEastmoneyRows(url);
  const timestamp = new Date().toISOString();

  return rows.flatMap((row) => {
    const descriptor = findIndexDescriptor(row.f12);
    const price = numberField(row.f2);
    if (!descriptor || price === null) return [];
    return [
      {
        market: descriptor.market,
        indexName: textField(row.f14) ?? descriptor.name,
        price,
        changePct: numberField(row.f3) ?? 0,
        turnover: numberField(row.f6),
        timestamp,
        source: SOURCE,
      },
    ];
  });
}

async function fetchSectorRows(): Promise<EastmoneyRow[]> {
  const [industryRows, conceptRows] = await Promise.all([
    fetchBoardRows("m:90+t:2", "200"),
    fetchBoardRows("m:90+t:3", "500"),
  ]);
  return uniqueRowsBySecid([...industryRows, ...conceptRows]);
}

async function fetchBoardRows(fs: string, pageSize: string): Promise<EastmoneyRow[]> {
  const fields = "f12,f13,f14,f3,f6,f62";
  const url = eastmoneyUrl(PUSH2_BASE_URL, "/api/qt/clist/get", {
    fields,
    fid: "f62",
    fltt: "2",
    fs,
    invt: "2",
    np: "1",
    pn: "1",
    po: "1",
    pz: pageSize,
  });
  return fetchEastmoneyRows(url);
}

function uniqueRowsBySecid(rows: EastmoneyRow[]): EastmoneyRow[] {
  const uniqueRows = new Map<string, EastmoneyRow>();
  for (const row of rows) {
    const secid = rowSecid(row);
    if (secid && !uniqueRows.has(secid)) {
      uniqueRows.set(secid, row);
    }
  }
  return Array.from(uniqueRows.values());
}

async function fetchHistoricalSectorRotationRange(
  rows: EastmoneyRow[],
  startDate: string,
  endDate: string,
): Promise<SectorRotationSnapshot[]> {
  const results = await Promise.allSettled(
    rows
      .filter((row) => textField(row.f12) && textField(row.f14))
      .slice(0, HISTORICAL_BOARD_LIMIT)
      .map(async (row): Promise<SectorRotationSnapshot[]> => {
        const secid = rowSecid(row);
        const sector = textField(row.f14);
        if (!secid || !sector) return [];
        const klines = await fetchDailyKlines(secid, startDate, endDate);
        return klines.map((kline) => ({
          market: "CN",
          sector,
          date: kline.date,
          netFlow: null,
          changePct: kline.changePct,
          turnover: kline.turnover,
          ranking: null,
          source: `${SOURCE}:historical_kline`,
        }));
      }),
  );

  const snapshots = results.flatMap((result) =>
    result.status === "fulfilled" ? result.value : [],
  );
  return rankHistoricalSnapshots(snapshots);
}

async function fetchDailyKlines(
  secid: string,
  startDate: string,
  endDate: string,
): Promise<Array<{ date: string; changePct: number | null; turnover: number | null }>> {
  const url = eastmoneyUrl(PUSH2HIS_BASE_URL, "/api/qt/stock/kline/get", {
    secid,
    klt: "101",
    fqt: "1",
    beg: startDate.replaceAll("-", ""),
    end: endDate.replaceAll("-", ""),
    fields1: "f1,f2,f3,f4,f5,f6",
    fields2: "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61",
    rtntype: "6",
  });
  const response = await jsonp<EastmoneyKlineResponse>(url);
  const lines = Array.isArray(response.data?.klines) ? response.data.klines : [];
  return lines.flatMap((line) => {
    if (typeof line !== "string") return [];
    const fields = line.split(",");
    const date = fields[0];
    if (!date) return [];
    return [
      {
        date,
        changePct: numberField(fields[8]),
        turnover: numberField(fields[6]),
      },
    ];
  });
}

function rankHistoricalSnapshots(snapshots: SectorRotationSnapshot[]): SectorRotationSnapshot[] {
  const snapshotsByDate = new Map<string, SectorRotationSnapshot[]>();
  for (const snapshot of snapshots) {
    const existing = snapshotsByDate.get(snapshot.date) ?? [];
    existing.push(snapshot);
    snapshotsByDate.set(snapshot.date, existing);
  }

  return Array.from(snapshotsByDate.values()).flatMap((dailySnapshots) =>
    dailySnapshots
      .sort((a, b) => (b.changePct ?? -Infinity) - (a.changePct ?? -Infinity))
      .map((snapshot, index) => ({ ...snapshot, ranking: index + 1 })),
  );
}

function toCapitalFlowSnapshots(rows: EastmoneyRow[]): CapitalFlowSnapshot[] {
  if (rows.length === 0) return [];
  const flows = rows.map((row) => numberField(row.f62)).filter((value) => value !== null);
  const inflow = flows.filter((value) => value > 0).reduce((sum, value) => sum + value, 0);
  const outflow = Math.abs(
    flows.filter((value) => value < 0).reduce((sum, value) => sum + value, 0),
  );
  return [
    {
      market: "CN",
      date: todayKey(),
      category: "main_funds",
      inflow,
      outflow,
      netFlow: inflow - outflow,
      source: SOURCE,
    },
  ];
}

function toSectorRotationSnapshots(rows: EastmoneyRow[]): SectorRotationSnapshot[] {
  return rows.flatMap((row, index) => {
    const sector = textField(row.f14);
    if (!sector) return [];
    return [
      {
        market: "CN",
        sector,
        date: todayKey(),
        netFlow: numberField(row.f62),
        changePct: numberField(row.f3),
        turnover: numberField(row.f6),
        ranking: index + 1,
        source: SOURCE,
      },
    ];
  });
}

function eastmoneyUrl(baseUrl: string, path: string, params: Record<string, string>): string {
  const search = new URLSearchParams({ ...params, _: String(Date.now()), ut: EASTMONEY_UT });
  return `${baseUrl}${path}?${search.toString()}`;
}

async function fetchEastmoneyRows(url: string): Promise<EastmoneyRow[]> {
  const response = await jsonp<EastmoneyResponse>(url);
  return Array.isArray(response.data?.diff) ? response.data.diff : [];
}

function jsonp<T>(url: string): Promise<T> {
  return new Promise((resolve, reject) => {
    if (typeof document === "undefined") {
      reject(new Error("Browser document is not available"));
      return;
    }
    const callbackName = `__wealthfolioEastmoney${Date.now()}${Math.random().toString(36).slice(2)}`;
    const script = document.createElement("script");
    const timeout = window.setTimeout(
      () => cleanup(new Error("Eastmoney JSONP timeout")),
      JSONP_TIMEOUT_MS,
    );

    const cleanup = (error?: Error, data?: T) => {
      window.clearTimeout(timeout);
      script.remove();
      delete (window as unknown as Record<string, unknown>)[callbackName];
      error ? reject(error) : resolve(data as T);
    };

    (window as unknown as Record<string, (data: T) => void>)[callbackName] = (data) =>
      cleanup(undefined, data);
    script.onerror = () => cleanup(new Error("Eastmoney JSONP request failed"));
    script.src = appendCallback(url, callbackName);
    document.head.appendChild(script);
  });
}

function appendCallback(url: string, callbackName: string): string {
  const separator = url.includes("?") ? "&" : "?";
  return `${url}${separator}cb=${encodeURIComponent(callbackName)}`;
}

function rowSecid(row: EastmoneyRow): string | null {
  const code = textField(row.f12);
  if (!code) return null;
  const market = numberField(row.f13);
  if (market !== null) return `${market}.${code}`;
  if (code.startsWith("BK")) return `90.${code}`;
  if (code.startsWith("6")) return `1.${code}`;
  return `0.${code}`;
}

function findIndexDescriptor(code?: string): IndexDescriptor | undefined {
  const cleanCode = textField(code);
  return INDEX_DESCRIPTORS.find((item) => cleanCode === item.code || cleanCode === item.secid);
}

function numberField(value: number | string | undefined): number | null {
  if (typeof value === "number" && Number.isFinite(value)) return value;
  if (typeof value !== "string" || value.trim() === "" || value === "-") return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function textField(value: string | undefined): string | null {
  const text = value?.trim();
  return text && text !== "-" ? text : null;
}

function todayKey(): string {
  return new Date().toISOString().slice(0, 10);
}
