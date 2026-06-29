import type { Exchange } from "./types";

/**
 * Normalize a Chinese A-share stock code into a 6-digit numeric code
 * and detect the exchange.
 *
 * Rules:
 *  - 600xxx / 601xxx / 603xxx / 605xxx / 688xxx → SH
 *  - 000xxx / 001xxx / 002xxx / 003xxx / 300xxx / 301xxx → SZ
 *  - 8xxxxx / 4xxxxx / 9xxxxx → BJ
 *  - Prefix "SH"/"SZ"/"BJ" is stripped.
 */
export function normalizeStockCode(raw: string): { code: string; exchange: Exchange } {
  const s = raw.trim().toUpperCase();

  // US stocks: alphabetic tickers like AAPL, TSLA, MU
  if (
    /^[A-Z]{1,5}(\.?[A-Z]{1,3})?$/.test(s) &&
    !s.startsWith("SH") &&
    !s.startsWith("SZ") &&
    !s.startsWith("BJ") &&
    !s.startsWith("HK")
  ) {
    // Check it's not a 6-digit Chinese code
    if (!/^\d{6}$/.test(s.replace(/^[A-Z]+/, ""))) {
      return { code: s, exchange: "US" };
    }
  }

  // Strip known prefixes
  let code = s;
  let prefixExchange: Exchange | undefined;
  if (s.startsWith("SH")) {
    code = s.slice(2);
    prefixExchange = "SH";
  } else if (s.startsWith("SZ")) {
    code = s.slice(2);
    prefixExchange = "SZ";
  } else if (s.startsWith("BJ")) {
    code = s.slice(2);
    prefixExchange = "BJ";
  } else if (s.startsWith("HK")) {
    code = s.slice(2);
    prefixExchange = "HK";
  }

  // Ensure it's numeric and 6 digits for A-shares
  if (!/^\d{6}$/.test(code)) {
    const match = /(\d{6})$/.exec(code);
    code = match ? match[1] : code.padStart(6, "0");
  }

  const first = code.charAt(0);

  let exchange: Exchange;
  if (prefixExchange) {
    exchange = prefixExchange;
  } else if (first === "6") {
    exchange = "SH";
  } else if (first === "0" || first === "3") {
    exchange = "SZ";
  } else if (["8", "4", "9"].includes(first)) {
    exchange = "BJ";
  } else {
    exchange = "UNKNOWN";
  }

  return { code, exchange };
}

/**
 * Build Eastmoney secid from stock code.
 * Shanghai: 1.xxxxxx
 * Shenzhen:  0.xxxxxx
 * Beijing:   0.xxxxxx (Eastmoney treats BJ same as SZ prefix)
 */
export function buildEastMoneySecid(code: string, exchange: Exchange): string {
  if (exchange === "SH") return `1.${code}`;
  if (exchange === "HK") return `116.${code}`;
  if (exchange === "US") return `105.${code}`;
  return `0.${code}`; // SZ & BJ both use 0.
}
