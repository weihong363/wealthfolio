import { describe, it, expect } from "vitest";
import { normalizeStockCode, buildEastMoneySecid } from "../code-utils";

describe("normalizeStockCode", () => {
  it("recognizes SH prefix", () => {
    expect(normalizeStockCode("SH600519")).toEqual({ code: "600519", exchange: "SH" });
  });

  it("recognizes SZ prefix", () => {
    expect(normalizeStockCode("SZ300308")).toEqual({ code: "300308", exchange: "SZ" });
  });

  it("recognizes 6-prefix as SH", () => {
    expect(normalizeStockCode("600519")).toEqual({ code: "600519", exchange: "SH" });
  });

  it("recognizes 0-prefix as SZ", () => {
    expect(normalizeStockCode("000001")).toEqual({ code: "000001", exchange: "SZ" });
  });

  it("recognizes 3-prefix as SZ", () => {
    expect(normalizeStockCode("300750")).toEqual({ code: "300750", exchange: "SZ" });
  });

  it("recognizes 688 as SH (STAR board)", () => {
    expect(normalizeStockCode("688981")).toEqual({ code: "688981", exchange: "SH" });
  });

  it("recognizes 8xxx as BJ", () => {
    expect(normalizeStockCode("833819")).toEqual({ code: "833819", exchange: "BJ" });
  });

  it("strips extra characters and extracts 6 digits", () => {
    expect(normalizeStockCode("sh.600519")).toEqual({ code: "600519", exchange: "SH" });
  });

  it("handles lower case input", () => {
    expect(normalizeStockCode("sz300308")).toEqual({ code: "300308", exchange: "SZ" });
  });

  it("recognizes US ticker", () => {
    expect(normalizeStockCode("AAPL")).toEqual({ code: "AAPL", exchange: "US" });
  });

  it("recognizes US ticker with dot", () => {
    expect(normalizeStockCode("BRK.B")).toEqual({ code: "BRK.B", exchange: "US" });
  });
});

describe("buildEastMoneySecid", () => {
  it("uses 1. prefix for SH", () => {
    expect(buildEastMoneySecid("600519", "SH")).toBe("1.600519");
  });

  it("uses 0. prefix for SZ", () => {
    expect(buildEastMoneySecid("300308", "SZ")).toBe("0.300308");
  });

  it("uses 0. prefix for BJ", () => {
    expect(buildEastMoneySecid("833819", "BJ")).toBe("0.833819");
  });

  it("roundtrip: normalize → build", () => {
    const { code, exchange } = normalizeStockCode("SH600519");
    expect(buildEastMoneySecid(code, exchange)).toBe("1.600519");
  });

  it("builds US secid", () => {
    expect(buildEastMoneySecid("AAPL", "US")).toBe("105.AAPL");
  });
});
