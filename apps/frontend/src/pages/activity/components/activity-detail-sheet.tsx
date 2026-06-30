import { ActivityStatus, ActivityTypeNames, SUBTYPE_DISPLAY_NAMES } from "@/lib/constants";
import { parseOccSymbol } from "@/lib/occ-symbol";
import type { ActivityDetails } from "@/lib/types";
import {
  Badge,
  Button,
  Icons,
  Separator,
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from "@wealthfolio/ui";
import { AmountDisplay } from "@wealthfolio/ui/components/financial/amount-display";
import { format } from "date-fns";
import { useTranslation } from "react-i18next";

interface ActivityDetailSheetProps {
  activity: ActivityDetails | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

// Status display configuration
const STATUS_CONFIG: Record<
  string,
  { labelKey: string; variant: "default" | "secondary" | "outline" | "destructive" }
> = {
  [ActivityStatus.POSTED]: { labelKey: "activities.grid.status.posted", variant: "default" },
  [ActivityStatus.PENDING]: { labelKey: "activities.grid.status.pending", variant: "secondary" },
  [ActivityStatus.DRAFT]: { labelKey: "activities.grid.status.draft", variant: "outline" },
  [ActivityStatus.VOID]: { labelKey: "activities.grid.status.void", variant: "destructive" },
};

interface DetailRowProps {
  label: string;
  value: React.ReactNode;
  icon?: React.ReactNode;
}

function DetailRow({ label, value, icon }: DetailRowProps) {
  return (
    <div className="flex items-start justify-between gap-4 py-2">
      <div className="text-muted-foreground flex items-center gap-2 text-sm">
        {icon}
        <span>{label}</span>
      </div>
      <div className="text-right text-sm font-medium">{value}</div>
    </div>
  );
}

interface DetailSectionProps {
  title: string;
  icon?: React.ReactNode;
  children: React.ReactNode;
}

function DetailSection({ title, icon, children }: DetailSectionProps) {
  return (
    <div className="space-y-1">
      <div className="flex items-center gap-2 pb-2">
        {icon}
        <h4 className="text-sm font-semibold">{title}</h4>
      </div>
      <div className="bg-muted/30 rounded-lg border p-3">{children}</div>
    </div>
  );
}

export function ActivityDetailSheet({ activity, open, onOpenChange }: ActivityDetailSheetProps) {
  const { t } = useTranslation();
  if (!activity) return null;

  const statusConfig = activity.status
    ? STATUS_CONFIG[activity.status] || {
        labelKey: activity.status,
        variant: "default" as const,
      }
    : null;

  const subtypeDisplay = activity.subtype
    ? SUBTYPE_DISPLAY_NAMES[activity.subtype] || activity.subtype
    : null;

  const formatDate = (date: Date | string | undefined) => {
    if (!date) return "—";
    const d = typeof date === "string" ? new Date(date) : date;
    return format(d, "PPpp");
  };

  const formatShortDate = (date: Date | string | undefined) => {
    if (!date) return "—";
    const d = typeof date === "string" ? new Date(date) : date;
    return format(d, "PP");
  };

  // Parse OCC symbol for option activities
  const isOption = activity.instrumentType === "OPTION";
  const parsedOption = isOption ? parseOccSymbol(activity.assetSymbol ?? "") : null;

  // Format option expiration for display (YYYY-MM-DD → "Mar 29, 2025")
  const optionExpirationDisplay = parsedOption?.expiration
    ? format(new Date(parsedOption.expiration + "T12:00:00"), "PP")
    : undefined;

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent side="right" className="w-full overflow-y-auto sm:max-w-lg">
        <SheetHeader className="pb-4">
          <SheetTitle className="flex items-center gap-3">
            <div className="bg-primary/10 flex h-10 w-10 items-center justify-center rounded-full">
              <Icons.Receipt className="text-primary h-5 w-5" />
            </div>
            <div className="flex flex-col items-start">
              <span>{t("activities.details.title")}</span>
              <span className="text-muted-foreground text-xs font-normal">
                {parsedOption
                  ? parsedOption.underlying
                  : activity.assetSymbol || t("activities.details.cashTransaction")}
              </span>
            </div>
          </SheetTitle>
        </SheetHeader>

        <div className="space-y-6 pb-6 md:pb-8">
          {/* Header Summary */}
          <div className="from-primary/5 to-primary/10 rounded-xl border bg-gradient-to-br p-4">
            <div className="flex items-start justify-between">
              <div>
                <div className="text-muted-foreground mb-1 text-xs uppercase tracking-wide">
                  {t(`activityManager.types.${activity.activityType.toLowerCase()}`, {
                    defaultValue: ActivityTypeNames[activity.activityType] || activity.activityType,
                  })}
                </div>
                {parsedOption ? (
                  <>
                    <div className="text-xl font-bold">{parsedOption.underlying}</div>
                    <div className="text-muted-foreground text-sm">
                      {optionExpirationDisplay} ${parsedOption.strikePrice}{" "}
                      {parsedOption.optionType}
                    </div>
                  </>
                ) : (
                  <>
                    {activity.assetSymbol && (
                      <div className="text-xl font-bold">{activity.assetSymbol}</div>
                    )}
                    {activity.assetName && (
                      <div className="text-muted-foreground text-sm">{activity.assetName}</div>
                    )}
                  </>
                )}
              </div>
              <div className="flex flex-col items-end gap-2">
                {statusConfig && (
                  <Badge variant={statusConfig.variant}>
                    {t(statusConfig.labelKey, { defaultValue: activity.status })}
                  </Badge>
                )}
                {activity.needsReview && (
                  <Badge variant="outline" className="border-amber-500 text-amber-600">
                    <Icons.AlertCircle className="mr-1 h-3 w-3" />
                    {t("spending.transactions.status.needsReview")}
                  </Badge>
                )}
              </div>
            </div>
            <Separator className="my-3" />
            <div className="grid grid-cols-2 gap-4">
              <div>
                <div className="text-muted-foreground text-xs">{t("activities.table.date")}</div>
                <div className="font-medium">{formatShortDate(activity.date)}</div>
              </div>
              <div className="text-right">
                <div className="text-muted-foreground text-xs">
                  {t("activityManager.form.totalCredit")}
                </div>
                <div className="text-lg font-bold">
                  <AmountDisplay value={Number(activity.amount)} currency={activity.currency} />
                </div>
              </div>
            </div>
          </div>

          {/* Transaction Details */}
          <DetailSection
            title={t("activities.details.transaction")}
            icon={<Icons.ArrowLeftRight className="h-4 w-4" />}
          >
            <DetailRow
              label={t("activities.type")}
              value={
                <Badge variant="outline">
                  {t(`activityManager.types.${activity.activityType.toLowerCase()}`, {
                    defaultValue: ActivityTypeNames[activity.activityType] || activity.activityType,
                  })}
                </Badge>
              }
            />
            {subtypeDisplay && <DetailRow label={t("activityManager.form.subtype")} value={subtypeDisplay} />}
            <DetailRow label={t("activities.grid.dateTime")} value={formatDate(activity.date)} />
            <DetailRow label={t("activities.table.account")} value={activity.accountName} />
          </DetailSection>

          {/* Option Contract Details */}
          {parsedOption && (
            <DetailSection title={t("activities.details.optionContract")} icon={<Icons.BarChart className="h-4 w-4" />}>
              <DetailRow label={t("activities.details.underlying")} value={parsedOption.underlying} />
              <DetailRow
                label={t("activities.type")}
                value={<Badge variant="outline">{parsedOption.optionType}</Badge>}
              />
              <DetailRow
                label={t("activities.details.strikePrice")}
                value={
                  <AmountDisplay value={parsedOption.strikePrice} currency={activity.currency} />
                }
              />
              <DetailRow label={t("activities.details.expiration")} value={optionExpirationDisplay} />
              <DetailRow label={t("activities.details.occSymbol")} value={activity.assetSymbol} />
            </DetailSection>
          )}

          {/* Financial Details */}
          <DetailSection title={t("activities.details.financialDetails")} icon={<Icons.DollarSign className="h-4 w-4" />}>
            {Number(activity.quantity) !== 0 && (
              <DetailRow
                label={isOption ? t("activities.units.contractsLabel") : t("activities.table.quantity")}
                value={Number(activity.quantity).toLocaleString(undefined, {
                  maximumFractionDigits: 8,
                })}
              />
            )}
            {Number(activity.unitPrice) !== 0 && (
              <DetailRow
                label={isOption ? t("activityManager.form.premiumShare") : t("activities.details.unitPrice")}
                value={
                  <AmountDisplay value={Number(activity.unitPrice)} currency={activity.currency} />
                }
              />
            )}
            <DetailRow
              label={isOption ? t("activities.details.totalPremium") : t("activityManager.form.totalCredit")}
              value={<AmountDisplay value={Number(activity.amount)} currency={activity.currency} />}
            />
            {Number(activity.fee) !== 0 && (
              <DetailRow
                label={t("activities.table.fee")}
                value={<AmountDisplay value={Number(activity.fee)} currency={activity.currency} />}
              />
            )}
            {activity.fxRate && (
              <DetailRow
                label={t("activityManager.form.fxRate")}
                value={Number(activity.fxRate).toLocaleString(undefined, {
                  maximumFractionDigits: 8,
                })}
              />
            )}
            <DetailRow label={t("activities.table.currency")} value={activity.currency} />
            {activity.accountCurrency !== activity.currency && (
              <DetailRow label={t("activityManager.form.accountCurrency")} value={activity.accountCurrency} />
            )}
          </DetailSection>

          {/* Comment */}
          {activity.comment && (
            <DetailSection title={t("activityManager.form.notes")} icon={<Icons.FileText className="h-4 w-4" />}>
              <p className="whitespace-pre-wrap text-sm">{activity.comment}</p>
            </DetailSection>
          )}

          {/* Metadata */}
          <DetailSection title={t("activities.details.recordInfo")} icon={<Icons.Info className="h-4 w-4" />}>
            <DetailRow label={t("activities.details.created")} value={formatDate(activity.createdAt)} />
            <DetailRow label={t("activities.details.updated")} value={formatDate(activity.updatedAt)} />
          </DetailSection>
        </div>

        {/* Mobile close button */}
        <div className="bg-background border-t p-4 md:hidden">
          <Button className="w-full" onClick={() => onOpenChange(false)}>
            {t("common.close")}
          </Button>
        </div>
      </SheetContent>
    </Sheet>
  );
}
