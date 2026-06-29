import { describe, it, expect } from "vitest";
import {
  mapF10Industry,
  mapF10Concepts,
  mapF10Boards,
  mapBoardQuote,
  mapQuoteProfile,
  computeMetrics,
} from "../mapper";
import type { StockConceptInfo, StockIndustryInfo } from "../types";

describe("mapF10Industry", () => {
  it("returns undefined for null", () => {
    expect(mapF10Industry(null)).toBeUndefined();
  });

  it("returns undefined for empty ssbk", () => {
    expect(mapF10Industry({ ssbk: [] })).toBeUndefined();
  });

  it("picks primary industry (BOARD_RANK=1)", () => {
    const result = mapF10Industry({
      ssbk: [
        {
          BOARD_CODE: "1215",
          BOARD_NAME: "通信",
          BOARD_RANK: 1,
        },
        {
          BOARD_CODE: "BK0900",
          BOARD_NAME: "光通信",
          BOARD_RANK: 2,
        },
      ],
    });
    expect(result?.boardCode).toBe("1215");
    expect(result?.boardName).toBe("通信");
    expect(result?.rankInBoard).toBe(1);
  });
});

describe("mapF10Concepts", () => {
  it("returns empty for null", () => {
    expect(mapF10Concepts(null)).toEqual([]);
  });

  it("maps hxtc keywords to concepts", () => {
    const result = mapF10Concepts({
      hxtc: [
        { KEYWORD: "光通信", MAINPOINT: 1 },
        { KEYWORD: "5G", MAINPOINT: 2 },
      ],
    });
    expect(result).toHaveLength(2);
    expect(result[0].name).toBe("光通信");
    expect(result[1].name).toBe("5G");
  });

  it("handles empty hxtc", () => {
    expect(mapF10Concepts({ hxtc: [] })).toEqual([]);
  });
});

describe("mapF10Boards", () => {
  it("classifies industry boards by rank", () => {
    const result = mapF10Boards({
      ssbk: [
        { BOARD_CODE: "1215", BOARD_NAME: "通信", BOARD_RANK: 1 },
        { BOARD_CODE: "BK01", BOARD_NAME: "光通信概念", BOARD_RANK: 2 },
      ],
    });
    expect(result[0].type).toBe("industry");
    expect(result[0].name).toBe("通信");
  });

  it("returns empty for null", () => {
    expect(mapF10Boards(null)).toEqual([]);
  });
});

describe("mapBoardQuote", () => {
  it("returns empty for null", () => {
    expect(mapBoardQuote(null)).toEqual({});
  });

  it("maps board quote data", () => {
    const result = mapBoardQuote({
      data: { diff: [{ f3: 2.5, f62: 1e8, f66: 5e9 }] },
    });
    expect(result).toEqual({
      changePercent: 2.5,
      mainNetInflow: 1e8,
      amount: 5e9,
    });
  });
});

describe("mapQuoteProfile", () => {
  it("returns empty for null data", () => {
    const result = mapQuoteProfile({ data: undefined });
    expect(result.concepts).toEqual([]);
    expect(result.boards).toEqual([]);
    expect(result.industry).toBeUndefined();
  });

  it("extracts industry, concepts, region from extended quote", () => {
    const result = mapQuoteProfile({
      data: {
        f127: "消费电子",
        f128: "美国",
        f129: "人工智能,AR,智能手机",
        f57: "Apple Inc",
        f43: 15000,
      },
    });
    expect(result.industry?.boardName).toBe("消费电子");
    expect(result.concepts).toHaveLength(3);
    expect(result.concepts[0].name).toBe("人工智能");
    expect(result.boards).toHaveLength(5); // 1 industry + 1 region + 3 concepts
    expect(result.boards.find((b) => b.type === "region")?.name).toBe("美国");
  });

  it("filters empty concept names", () => {
    const result = mapQuoteProfile({
      data: {
        f129: "AI,, , 芯片",
      },
    });
    expect(result.concepts).toHaveLength(2);
    expect(result.concepts[0].name).toBe("AI");
    expect(result.concepts[1].name).toBe("芯片");
  });
});

describe("computeMetrics", () => {
  it("computes basic metrics for empty concepts", () => {
    const metrics = computeMetrics([]);
    expect(metrics.conceptCount).toBe(0);
    expect(metrics.hotConceptCount).toBe(0);
    expect(metrics.dominantTheme).toBeUndefined();
  });

  it("detects hot concepts", () => {
    const concepts: StockConceptInfo[] = [{ name: "人工智能", isHot: true }, { name: "普通概念" }];
    const metrics = computeMetrics(concepts);
    expect(metrics.hotConceptCount).toBe(1);
    expect(metrics.dominantTheme).toBe("人工智能");
  });

  it("includes industry in fundRelevanceTags", () => {
    const industry: StockIndustryInfo = { boardName: "通信设备" };
    const metrics = computeMetrics([{ name: "AI" }], industry);
    expect(metrics.fundRelevanceTags).toContain("通信设备");
    expect(metrics.fundRelevanceTags).toContain("AI");
  });

  it("computes concept momentum score", () => {
    const concepts: StockConceptInfo[] = [
      { name: "AI", changePercent: 5 },
      { name: "芯片", changePercent: -2 },
    ];
    const metrics = computeMetrics(concepts);
    expect(metrics.conceptMomentumScore).toBe(1.5);
  });
});
