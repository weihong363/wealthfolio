import { Badge, Button, Icons } from "@wealthfolio/ui";
import { Input } from "@wealthfolio/ui/components/ui/input";
import { Label } from "@wealthfolio/ui/components/ui/label";
import { Sheet, SheetContent, SheetHeader, SheetTitle } from "@wealthfolio/ui/components/ui/sheet";
import { Textarea } from "@wealthfolio/ui/components/ui/textarea";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { HoldingClassificationOverride, PortfolioFundLookthroughHolding } from "../types";
import { cleanAssetName, holdingClassificationKey } from "../utils";

export function HoldingClassificationSheet({
  holding,
  override,
  open,
  onOpenChange,
  onSave,
  onClear,
}: {
  holding: PortfolioFundLookthroughHolding | null;
  override?: HoldingClassificationOverride;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSave: (
    holding: PortfolioFundLookthroughHolding,
    input: { sector?: string; industry?: string; themeTags?: string[] },
  ) => void;
  onClear: (holding: PortfolioFundLookthroughHolding) => void;
}) {
  const { t } = useTranslation();
  const [industry, setIndustry] = useState("");
  const [themes, setThemes] = useState("");

  useEffect(() => {
    if (!holding || !open) return;
    setIndustry(override?.industry ?? override?.sector ?? holding.industry ?? holding.sector ?? "");
    setThemes((override?.themeTags ?? holding.themeTags).join(", "));
  }, [holding, open, override]);

  if (!holding) return null;

  const hasOverride = !!override;

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent>
        <SheetHeader>
          <SheetTitle className="text-lg">{t("fundResearch.editClassification")}</SheetTitle>
          <p className="text-muted-foreground text-sm">
            {cleanAssetName(holding.assetName)} · {holdingClassificationKey(holding)}
          </p>
        </SheetHeader>

        <div className="mt-6 space-y-5">
          <div className="space-y-2">
            <Label htmlFor="fund-holding-industry">{t("fundResearch.industry")}</Label>
            <Input
              id="fund-holding-industry"
              value={industry}
              onChange={(event) => setIndustry(event.target.value)}
              placeholder={t("fundResearch.industryPlaceholder")}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="fund-holding-themes">{t("fundResearch.themes")}</Label>
            <Textarea
              id="fund-holding-themes"
              value={themes}
              onChange={(event) => setThemes(event.target.value)}
              placeholder={t("fundResearch.themesPlaceholder")}
              rows={4}
            />
            <p className="text-muted-foreground text-xs">{t("fundResearch.themesHelper")}</p>
          </div>

          <div className="space-y-2">
            <p className="text-muted-foreground text-xs">{t("fundResearch.currentSourceValue")}</p>
            <div className="flex flex-wrap gap-1">
              {(holding.industry ?? holding.sector) && (
                <Badge variant="secondary">{holding.industry ?? holding.sector}</Badge>
              )}
              {holding.themeTags.map((tag) => (
                <Badge key={tag} variant="secondary">
                  {tag}
                </Badge>
              ))}
              {!holding.industry && !holding.sector && holding.themeTags.length === 0 && (
                <span className="text-muted-foreground text-xs">
                  {t("fundResearch.unclassified")}
                </span>
              )}
            </div>
          </div>

          <div className="flex gap-2">
            <Button
              className="flex-1"
              onClick={() => {
                onSave(holding, {
                  sector: industry,
                  industry,
                  themeTags: splitThemes(themes),
                });
                onOpenChange(false);
              }}
            >
              <Icons.Save className="mr-2 h-4 w-4" />
              {t("common.save")}
            </Button>
            <Button
              type="button"
              variant="outline"
              disabled={!hasOverride}
              onClick={() => {
                onClear(holding);
                onOpenChange(false);
              }}
            >
              <Icons.Undo className="mr-2 h-4 w-4" />
              {t("fundResearch.resetClassification")}
            </Button>
          </div>
        </div>
      </SheetContent>
    </Sheet>
  );
}

function splitThemes(value: string): string[] {
  return value
    .split(/[,，、\n]/)
    .map((item) => item.trim())
    .filter(Boolean);
}
