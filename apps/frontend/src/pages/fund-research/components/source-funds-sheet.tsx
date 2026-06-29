import { AmountDisplay } from "@wealthfolio/ui";
import { Sheet, SheetContent, SheetHeader, SheetTitle } from "@wealthfolio/ui/components/ui/sheet";
import { useTranslation } from "react-i18next";
import type { PortfolioFundLookthroughHolding } from "../types";
import { cleanAssetName } from "../utils";

export function SourceFundsSheet({
  holding,
  open,
  onOpenChange,
}: {
  holding: PortfolioFundLookthroughHolding | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { t } = useTranslation();
  if (!holding) return null;

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent>
        <SheetHeader>
          <SheetTitle className="text-lg">{cleanAssetName(holding.assetName)}</SheetTitle>
          <p className="text-muted-foreground text-sm">
            {t("fundResearch.indirectExposure")} ·{" "}
            {t("fundResearch.fundsCount", { count: holding.sourceFunds.length })}
          </p>
        </SheetHeader>
        <div className="mt-6 space-y-3">
          {holding.sourceFunds.map((sf) => (
            <div key={sf.fundCode} className="bg-muted/30 rounded-lg border p-3">
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium">{sf.fundName ?? sf.fundCode}</span>
                <span className="text-muted-foreground text-xs">{sf.fundCode}</span>
              </div>
              <div className="text-muted-foreground mt-2 grid grid-cols-2 gap-1 text-xs">
                <span>{t("fundResearch.fundMarketValue")}:</span>
                <span className="text-right">
                  <AmountDisplay value={sf.fundMarketValueBase} currency="CNY" />
                </span>
                <span>{t("fundResearch.fundWeight")}:</span>
                <span className="text-right">{sf.fundHoldingWeightPct.toFixed(2)}%</span>
                <span>{t("fundResearch.contribution")}:</span>
                <span className="text-right">
                  <AmountDisplay value={sf.exposureValueBase} currency="CNY" />
                </span>
              </div>
            </div>
          ))}
        </div>
      </SheetContent>
    </Sheet>
  );
}
