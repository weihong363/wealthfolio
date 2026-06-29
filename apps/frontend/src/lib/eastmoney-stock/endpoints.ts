// Eastmoney API endpoints for stock theme/industry data.
// Centralised so endpoint changes only touch one file.

const EM_QUOTE_BASE = "https://push2.eastmoney.com/api/qt/stock/get";
const EM_BOARD_BASE = "https://push2.eastmoney.com/api/qt/clist/get";
const EM_KLINE_BASE = "https://push2his.eastmoney.com/api/qt/stock/kline/get";
const EM_F10_BASE = "https://emweb.securities.eastmoney.com/PC_HSF10";

/**
 * Real-time quote for a single stock (basic fields).
 */
export function stockQuoteUrl(secid: string): string {
  const fields = "f43,f44,f45,f46,f47,f48,f50,f57,f58,f116,f169,f170";
  return `${EM_QUOTE_BASE}?secid=${encodeURIComponent(secid)}&fields=${fields}`;
}

/**
 * Extended quote with industry/concept fields (for US/HK stocks).
 * Returns f127=industry, f128=region, f129=concepts (comma-sep).
 */
export function stockQuoteExtendedUrl(secid: string): string {
  const fields = "f43,f47,f48,f50,f57,f58,f116,f127,f128,f129,f169,f170";
  return `${EM_QUOTE_BASE}?secid=${encodeURIComponent(secid)}&fields=${fields}`;
}

export function stockKlineUrl(secid: string, beg: string, end = "20500101"): string {
  const fields1 = "f1,f2,f3,f4,f5,f6";
  const fields2 = "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61";
  return `${EM_KLINE_BASE}?secid=${encodeURIComponent(secid)}&klt=101&fqt=1&beg=${beg}&end=${end}&fields1=${fields1}&fields2=${fields2}`;
}

/**
 * Stock F10 CoreConception page — returns ssbk (boards), hxtc (core themes),
 * and zyzb (key indicators).
 */
export function stockF10CoreConceptionUrl(stockCode: string, exchange: "SH" | "SZ"): string {
  const prefix = exchange === "SH" ? "SH" : "SZ";
  return `${EM_F10_BASE}/CoreConception/PageAjax?code=${prefix}${stockCode}`;
}

/**
 * Board real-time data (change%, net inflow, amount) for a given board code+type.
 * type: 1=industry, 2=concept
 */
export function boardQuoteUrl(boardCode: string, boardType: 1 | 2): string {
  const fields = "f2,f3,f4,f12,f14,f62,f66,f104,f105";
  return `${EM_BOARD_BASE}?pn=1&pz=1&po=0&np=1&fltt=2&invt=2&fid=f3&fs=m:90+t${boardType}+f:!50&fields=${fields}&secids=${encodeURIComponent(boardCode)}`;
}

/**
 * Stock F10 company info page (used as Referer).
 */
export function stockF10PageUrl(prefix: "SH" | "SZ", stockCode: string): string {
  return `${EM_F10_BASE}/CompanySurvey/CompanySurvey?code=${prefix}${stockCode}`;
}

/**
 * Board page URL (used as Referer).
 */
export function boardPageUrl(boardCode: string, _boardType: "industry" | "concept"): string {
  return `https://quote.eastmoney.com/bk/${encodeURIComponent(boardCode)}.html`;
}

export function stockPageUrl(stockCode: string, exchange: string): string {
  if (exchange === "US") return `https://quote.eastmoney.com/us/${stockCode}.html`;
  if (exchange === "HK") return `https://quote.eastmoney.com/hk/${stockCode}.html`;
  if (exchange === "BJ") return `https://quote.eastmoney.com/bj/${stockCode}.html`;
  if (exchange === "SH" && stockCode.startsWith("688")) {
    return `https://quote.eastmoney.com/kcb/${stockCode}.html`;
  }
  if (exchange === "SH") return `https://quote.eastmoney.com/sh${stockCode}.html`;
  if (exchange === "SZ") return `https://quote.eastmoney.com/sz${stockCode}.html`;
  return `https://quote.eastmoney.com/unify/r/${stockCode}`;
}
