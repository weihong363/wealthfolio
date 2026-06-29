import { AmountDisplay, Button, Card, formatPercent } from "@wealthfolio/ui";
import { Cell, Pie, PieChart, ResponsiveContainer } from "recharts";
import { useTranslation } from "react-i18next";

import type { FundResearchDistributionMode, LookthroughDistributionSlice } from "../types";

const COLORS = [
  "#2563eb",
  "#16a34a",
  "#f59e0b",
  "#dc2626",
  "#7c3aed",
  "#0891b2",
  "#db2777",
  "#65a30d",
  "#ea580c",
  "#4f46e5",
];

interface LookthroughDistributionChartProps {
  mode: FundResearchDistributionMode;
  onModeChange: (mode: FundResearchDistributionMode) => void;
  data: LookthroughDistributionSlice[];
}

export function LookthroughDistributionChart({
  mode,
  onModeChange,
  data,
}: LookthroughDistributionChartProps) {
  const { t } = useTranslation();
  const topSlices = data.slice(0, 10);

  return (
    <Card className="p-4">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <p className="text-sm font-medium">{t("fundResearch.distribution")}</p>
          <p className="text-muted-foreground text-xs">{t("fundResearch.distributionDesc")}</p>
        </div>
        <div className="flex flex-wrap gap-2">
          {(["sector", "theme", "holding"] as const).map((item) => (
            <Button
              key={item}
              size="xs"
              variant={mode === item ? "default" : "outline"}
              onClick={() => onModeChange(item)}
              className="h-7 text-xs"
            >
              {t(`fundResearch.distributionModes.${item}`)}
            </Button>
          ))}
        </div>
      </div>

      <div className="mt-4 grid gap-4 lg:grid-cols-[320px_1fr]">
        <div className="h-[260px] min-w-0">
          <ResponsiveContainer width="100%" height="100%">
            <PieChart>
              <Pie
                data={topSlices}
                dataKey="exposureValueBase"
                nameKey="name"
                innerRadius={56}
                outerRadius={104}
                paddingAngle={2}
                strokeWidth={0}
              >
                {topSlices.map((slice, index) => (
                  <Cell key={slice.name} fill={COLORS[index % COLORS.length]} />
                ))}
              </Pie>
            </PieChart>
          </ResponsiveContainer>
        </div>

        <div className="space-y-2">
          {topSlices.map((slice, index) => (
            <div
              key={slice.name}
              className="grid grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-3 text-sm"
            >
              <div className="flex min-w-0 items-center gap-2">
                <span
                  className="h-2.5 w-2.5 shrink-0 rounded-full"
                  style={{ backgroundColor: COLORS[index % COLORS.length] }}
                />
                <span className="truncate">{slice.name}</span>
              </div>
              <span className="text-muted-foreground text-right text-xs tabular-nums">
                <AmountDisplay value={slice.exposureValueBase} currency="CNY" />
              </span>
              <span className="w-14 text-right font-mono text-xs">
                {formatPercent(slice.weightPct / 100)}
              </span>
            </div>
          ))}
        </div>
      </div>
    </Card>
  );
}
