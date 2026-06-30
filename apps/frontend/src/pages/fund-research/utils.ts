import type {
  FundResearchDistributionMode,
  HoldingClassification,
  HoldingClassificationOverrides,
  LookthroughDistributionSlice,
  PortfolioFundLookthroughHolding,
} from "./types";

/**
 * Known stock names for assets where EastMoney doesn't provide profile data
 * (e.g. Japanese stocks, HK stocks with no EastMoney coverage).
 * Key is the cleaned asset name (ticker), value is the display name.
 */
const STOCK_NAME_OVERRIDES: Record<string, string> = {
  "285AJP": "铠侠 (Kioxia)",
};

/**
 * Strip HTML tags and common CSS class prefixes from Eastmoney-scraped asset names,
 * and apply known name overrides for stocks without EastMoney profiles.
 * e.g. "class='tol'宁德时代" -> "宁德时代"
 *      "<a class='tol'>中际旭创</a>" -> "中际旭创"
 *      "285AJP" -> "铠侠 (Kioxia)"
 */
export function cleanAssetName(raw: string): string {
  // Remove HTML tags: <a ...>, </a>, etc.
  let cleaned = raw.replace(/<[^>]*>/g, "");
  // Remove leftover class='xxx' / class="xxx" prefixes that may remain
  cleaned = cleaned.replace(/^class\s*=\s*['"][^'"]*['"]/, "");
  // Remove style='xxx' prefixes
  cleaned = cleaned.replace(/^style\s*=\s*['"][^'"]*['"]/, "");
  cleaned = cleaned.trim();

  // Apply known name overrides for stocks without EastMoney coverage
  const override = STOCK_NAME_OVERRIDES[cleaned];
  return override ?? cleaned;
}

export function classifyHolding(input: {
  assetName: string;
  assetType?: string;
  assetCode?: string | null;
  sector?: string;
  industry?: string;
  themeTags?: string[];
}): HoldingClassification {
  return {
    sectors: uniqueLabels([input.sector, input.industry].filter(isNonEmptyString)),
    themes: uniqueLabels(input.themeTags ?? []),
  };
}

export function extractHoldingStockCode(input: {
  assetCode?: string | null;
  assetName: string;
}): string | null {
  if (input.assetCode?.trim()) return input.assetCode.trim();

  const cleaned = cleanAssetName(input.assetName).replace(/\s+/g, "");
  const ticker = /^([A-Z]{1,5}(\.[A-Z]{1,3})?)/i.exec(cleaned);
  return ticker ? ticker[1] : null;
}

export function holdingClassificationKey(input: {
  assetCode?: string | null;
  assetName: string;
}): string {
  const code = extractHoldingStockCode(input);
  return (code ?? cleanAssetName(input.assetName)).trim().toUpperCase();
}

export function applyHoldingClassificationOverride<T extends PortfolioFundLookthroughHolding>(
  holding: T,
  overrides: HoldingClassificationOverrides,
): T {
  const override = overrides[holdingClassificationKey(holding)];
  if (!override) return holding;

  return {
    ...holding,
    sector: override.sector !== undefined ? emptyToUndefined(override.sector) : holding.sector,
    industry:
      override.industry !== undefined ? emptyToUndefined(override.industry) : holding.industry,
    themeTags: override.themeTags ?? holding.themeTags,
  };
}

export function buildLookthroughDistribution(
  holdings: PortfolioFundLookthroughHolding[],
  mode: FundResearchDistributionMode,
  fallbackLabel: string,
): LookthroughDistributionSlice[] {
  const values = new Map<string, number>();
  let total = 0;

  for (const holding of holdings) {
    const exposure = Math.max(0, holding.exposureValueBase);
    if (exposure <= 0) continue;

    const classification = classifyHolding(holding);
    const labels = labelsForMode(holding, classification, mode, fallbackLabel);
    const splitExposure = exposure / labels.length;
    total += exposure;

    for (const label of labels) {
      values.set(label, (values.get(label) ?? 0) + splitExposure);
    }
  }

  if (total <= 0) return [];

  return [...values.entries()]
    .map(([name, exposureValueBase]) => ({
      name,
      exposureValueBase,
      weightPct: (exposureValueBase / total) * 100,
    }))
    .sort((a, b) => b.exposureValueBase - a.exposureValueBase);
}

function labelsForMode(
  holding: PortfolioFundLookthroughHolding,
  classification: HoldingClassification,
  mode: FundResearchDistributionMode,
  fallbackLabel: string,
): string[] {
  if (mode === "holding") {
    return [cleanAssetName(holding.assetName) || fallbackLabel];
  }

  const labels = mode === "sector" ? classification.sectors : classification.themes;
  return labels.length > 0 ? labels : [fallbackLabel];
}

function uniqueLabels(labels: string[]): string[] {
  const seen = new Set<string>();
  const result: string[] = [];
  for (const label of labels.map((item) => item.trim()).filter(Boolean)) {
    if (seen.has(label)) continue;
    seen.add(label);
    result.push(label);
  }
  return result;
}

function isNonEmptyString(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

function emptyToUndefined(value: string): string | undefined {
  const cleaned = value.trim();
  return cleaned ? cleaned : undefined;
}
