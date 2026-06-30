import type {
  BrowserMarketIntelligenceSnapshots,
  CapitalFlowSnapshot,
  MarketSnapshot,
  SectorRotationSnapshot,
} from "@/pages/market-intelligence/types";

const PUSH2_BASE_URL = "https://push2.eastmoney.com";
const EASTMONEY_UT = "bd1d9ddb04089700cf9c27f6f7426281";
const JSONP_TIMEOUT_MS = 15_000;

interface EastmoneyResponse {
  data?: {
    diff?: EastmoneyRow[];
  };
}

interface EastmoneyRow {
  f2?: number | string;
  f3?: number | string;
  f6?: number | string;
  f12?: string;
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

export async function fetchEastmoneyMarketIntelligenceSnapshots(): Promise<BrowserMarketIntelligenceSnapshots> {
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

async function fetchMarketOverview(): Promise<MarketSnapshot[]> {
  const fields = "f12,f14,f2,f3,f4,f6";
  const secids = INDEX_DESCRIPTORS.map((item) => item.secid).join(",");
  const url = eastmoneyUrl("/api/qt/ulist.np/get", {
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
    return [{
      market: descriptor.market,
      indexName: textField(row.f14) ?? descriptor.name,
      price,
      changePct: numberField(row.f3) ?? 0,
      turnover: numberField(row.f6),
      timestamp,
      source: "eastmoney_browser",
    }];
  });
}

async function fetchSectorRows(): Promise<EastmoneyRow[]> {
  const fields = "f12,f14,f3,f6,f62";
  const url = eastmoneyUrl("/api/qt/clist/get", {
    fields,
    fid: "f62",
    fltt: "2",
    fs: "m:90+t:2",
    invt: "2",
    np: "1",
    pn: "1",
    po: "1",
    pz: "200",
  });
  return fetchEastmoneyRows(url);
}

function toCapitalFlowSnapshots(rows: EastmoneyRow[]): CapitalFlowSnapshot[] {
  if (rows.length === 0) return [];
  const flows = rows.map((row) => numberField(row.f62)).filter((value) => value !== null);
  const inflow = flows.filter((value) => value > 0).reduce((sum, value) => sum + value, 0);
  const outflow = Math.abs(flows.filter((value) => value < 0).reduce((sum, value) => sum + value, 0));
  return [{
    market: "CN",
    date: todayKey(),
    category: "main_funds",
    inflow,
    outflow,
    netFlow: inflow - outflow,
    source: "eastmoney_browser",
  }];
}

function toSectorRotationSnapshots(rows: EastmoneyRow[]): SectorRotationSnapshot[] {
  return rows.flatMap((row, index) => {
    const sector = textField(row.f14);
    if (!sector) return [];
    return [{
      market: "CN",
      sector,
      date: todayKey(),
      netFlow: numberField(row.f62),
      changePct: numberField(row.f3),
      turnover: numberField(row.f6),
      ranking: index + 1,
      source: "eastmoney_browser",
    }];
  });
}

function eastmoneyUrl(path: string, params: Record<string, string>): string {
  const search = new URLSearchParams({ ...params, _: String(Date.now()), ut: EASTMONEY_UT });
  return `${PUSH2_BASE_URL}${path}?${search.toString()}`;
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
    const timeout = window.setTimeout(() => cleanup(new Error("Eastmoney JSONP timeout")), JSONP_TIMEOUT_MS);

    const cleanup = (error?: Error, data?: T) => {
      window.clearTimeout(timeout);
      script.remove();
      delete (window as unknown as Record<string, unknown>)[callbackName];
      error ? reject(error) : resolve(data as T);
    };

    (window as unknown as Record<string, (data: T) => void>)[callbackName] = (data) => cleanup(undefined, data);
    script.onerror = () => cleanup(new Error("Eastmoney JSONP request failed"));
    script.src = appendCallback(url, callbackName);
    document.head.appendChild(script);
  });
}

function appendCallback(url: string, callbackName: string): string {
  const separator = url.includes("?") ? "&" : "?";
  return `${url}${separator}cb=${encodeURIComponent(callbackName)}`;
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
