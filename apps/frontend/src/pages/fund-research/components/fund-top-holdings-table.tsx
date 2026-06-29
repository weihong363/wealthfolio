import { Badge, formatPercent } from "@wealthfolio/ui";
import { useTranslation } from "react-i18next";
import type { FundTopHolding } from "../types";
import { classifyHolding, cleanAssetName } from "../utils";

export function FundTopHoldingsTable({ holdings }: { holdings: FundTopHolding[] }) {
  const { t } = useTranslation();
  return (
    <div className="overflow-x-auto">
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b text-left">
            <th className="text-muted-foreground px-2 py-2 text-xs font-medium">
              {t("fundResearch.rank")}
            </th>
            <th className="text-muted-foreground px-2 py-2 text-xs font-medium">
              {t("fundResearch.asset")}
            </th>
            <th className="text-muted-foreground hidden px-2 py-2 text-xs font-medium sm:table-cell">
              {t("fundResearch.code")}
            </th>
            <th className="text-muted-foreground hidden px-2 py-2 text-xs font-medium md:table-cell">
              {t("fundResearch.market")}
            </th>
            <th className="text-muted-foreground hidden px-2 py-2 text-xs font-medium md:table-cell">
              {t("fundResearch.type")}
            </th>
            <th className="text-muted-foreground hidden px-2 py-2 text-xs font-medium lg:table-cell">
              {t("fundResearch.sector")}
            </th>
            <th className="text-muted-foreground hidden px-2 py-2 text-xs font-medium xl:table-cell">
              {t("fundResearch.themes")}
            </th>
            <th className="text-muted-foreground px-2 py-2 text-right text-xs font-medium">
              {t("fundResearch.weight")}
            </th>
          </tr>
        </thead>
        <tbody>
          {holdings.map((h) => {
            const classification = classifyHolding(h);
            return (
              <tr key={`${h.rank}-${h.assetName}`} className="hover:bg-muted/30 border-b">
                <td className="px-2 py-2 text-xs">{h.rank ?? "-"}</td>
                <td className="max-w-[160px] truncate px-2 py-2 font-medium">
                  {cleanAssetName(h.assetName)}
                </td>
                <td className="text-muted-foreground hidden truncate px-2 py-2 text-xs sm:table-cell">
                  {h.assetCode ?? "-"}
                </td>
                <td className="text-muted-foreground hidden px-2 py-2 text-xs md:table-cell">
                  {h.market ?? "-"}
                </td>
                <td className="text-muted-foreground hidden px-2 py-2 text-xs capitalize md:table-cell">
                  {h.assetType}
                </td>
                <td className="hidden max-w-[180px] px-2 py-2 lg:table-cell">
                  <ClassificationBadges
                    labels={classification.sectors}
                    fallback={t("fundResearch.unclassified")}
                  />
                </td>
                <td className="hidden max-w-[220px] px-2 py-2 xl:table-cell">
                  <ClassificationBadges
                    labels={classification.themes}
                    fallback={t("fundResearch.unclassified")}
                  />
                </td>
                <td className="px-2 py-2 text-right font-mono text-xs">
                  {formatPercent(h.weightPct / 100)}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

function ClassificationBadges({ labels, fallback }: { labels: string[]; fallback: string }) {
  const displayLabels = labels.length > 0 ? labels.slice(0, 3) : [fallback];

  return (
    <div className="flex flex-wrap gap-1">
      {displayLabels.map((label) => (
        <Badge key={label} variant="secondary" className="max-w-28 truncate text-[11px]">
          {label}
        </Badge>
      ))}
    </div>
  );
}
