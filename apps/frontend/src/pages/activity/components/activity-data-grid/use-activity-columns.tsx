import { searchTicker } from "@/adapters";
import {
  isAssetBackedIncomeSubtype,
  isAssetIdentityRequired,
  isCashActivity,
} from "@/lib/activity-utils";
import {
  ActivityStatus,
  ActivityType,
  ActivityTypeNames,
  INSTRUMENT_TYPE_OPTIONS,
  getExchangeDisplayName,
  SUBTYPE_DISPLAY_NAMES,
  SUBTYPES_BY_ACTIVITY_TYPE,
} from "@/lib/constants";
import { parseOccSymbol } from "@/lib/occ-symbol";
import type { Account, ActivityDetails } from "@/lib/types";
import type { ColumnDef } from "@tanstack/react-table";
import { Badge, Checkbox, type SymbolSearchResult } from "@wealthfolio/ui";
import { useCallback, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { ActivityOperations } from "../activity-operations";
import { ActivityTypeBadge } from "../activity-type-badge";
import { StatusHeaderIndicator, StatusIndicator } from "./status-indicator";
import { isPendingReview, type LocalTransaction } from "./types";

// Status display names and colors
const STATUS_DISPLAY: Record<
  string,
  { labelKey: string; variant: "default" | "secondary" | "outline" | "destructive" }
> = {
  [ActivityStatus.POSTED]: { labelKey: "activities.grid.status.posted", variant: "default" },
  [ActivityStatus.PENDING]: { labelKey: "activities.grid.status.pending", variant: "secondary" },
  [ActivityStatus.DRAFT]: { labelKey: "activities.grid.status.draft", variant: "outline" },
  [ActivityStatus.VOID]: { labelKey: "activities.grid.status.void", variant: "destructive" },
};

const isTransferActivity = (activityType: string | undefined): boolean => {
  return activityType === ActivityType.TRANSFER_IN || activityType === ActivityType.TRANSFER_OUT;
};

const normalizeActivityToken = (value: string | null | undefined): string =>
  value?.trim().toUpperCase() ?? "";

const shouldDisplaySubtype = (
  transaction: LocalTransaction | undefined,
  activityType: string | undefined,
  subtype: string | null | undefined,
): boolean => {
  const normalizedSubtype = normalizeActivityToken(subtype);
  if (!normalizedSubtype) return false;

  const normalizedActivityType = normalizeActivityToken(activityType);
  return (
    normalizedSubtype !== normalizedActivityType || (!!transaction && isPendingReview(transaction))
  );
};

const getSubtypeDisplayLabel = (subtype: string, optionLabel?: string): string => {
  const normalizedSubtype = normalizeActivityToken(subtype);
  return optionLabel ?? SUBTYPE_DISPLAY_NAMES[normalizedSubtype] ?? subtype;
};

interface UseActivityColumnsOptions {
  accounts: Account[];
  onEditActivity: (activity: ActivityDetails) => void;
  onDuplicate: (activity: ActivityDetails) => void;
  onDelete: (activity: ActivityDetails) => void;
  onLinkTransfer?: (activity: ActivityDetails) => void;
  onUnlinkTransfer?: (activity: ActivityDetails) => void;
  /** Called when a symbol is selected from search, with the full result including exchangeMic */
  onSymbolSelect?: (rowIndex: number, result: SymbolSearchResult) => void;
  /** Called when user wants to create a custom asset. Opens a dialog to collect asset metadata. */
  onCreateCustomAsset?: (rowIndex: number, symbol: string) => void;
}

/**
 * Hook to create column definitions for the activity data grid
 */
export function useActivityColumns({
  accounts,
  onEditActivity,
  onDuplicate,
  onDelete,
  onLinkTransfer,
  onUnlinkTransfer,
  onSymbolSelect,
  onCreateCustomAsset,
}: UseActivityColumnsOptions) {
  const { t } = useTranslation();
  const activityTypeOptions = useMemo(
    () =>
      (Object.values(ActivityType) as ActivityType[]).map((type) => ({
        value: type,
        label: t(`activityManager.types.${type.toLowerCase()}`, {
          defaultValue: ActivityTypeNames[type],
        }),
      })),
    [t],
  );

  const accountOptions = useMemo(
    () =>
      accounts.map((account) => ({
        value: account.id,
        label: account.name,
      })),
    [accounts],
  );

  const handleSymbolSearch = useCallback(async (query: string): Promise<SymbolSearchResult[]> => {
    return searchTicker(query);
  }, []);

  const columns = useMemo<ColumnDef<LocalTransaction>[]>(
    () => [
      // === Pinned left (always visible) ===
      // 1. Select
      {
        id: "select",
        header: ({ table }) => (
          <Checkbox
            checked={
              table.getIsAllRowsSelected() || (table.getIsSomeRowsSelected() && "indeterminate")
            }
            onCheckedChange={(checked) => table.toggleAllRowsSelected(Boolean(checked))}
            aria-label={t("activities.grid.selectAllRows")}
          />
        ),
        cell: ({ row }) => (
          <Checkbox
            checked={row.getIsSelected()}
            onCheckedChange={(checked) => row.toggleSelected(Boolean(checked))}
            aria-label={t("activities.grid.selectRow")}
          />
        ),
        size: 40,
        minSize: 40,
        maxSize: 40,
        enableSorting: false,
        enableResizing: false,
        enableHiding: false,
        enablePinning: false,
      },
      // 2. Status indicator
      {
        id: "status",
        header: ({ table }) => {
          const hasRowsToReview = table
            .getRowModel()
            .rows.some((row) => isPendingReview(row.original));
          return <StatusHeaderIndicator hasRowsToReview={hasRowsToReview} />;
        },
        size: 32,
        minSize: 32,
        maxSize: 32,
        enableResizing: false,
        enableSorting: false,
        enableHiding: false,
        enablePinning: false,
        cell: ({ row }) => <StatusIndicator transaction={row.original} />,
      },
      // 3. Date & Time (primary sort key)
      {
        id: "date",
        accessorKey: "date",
        header: t("activities.grid.dateTime"),
        size: 180,
        meta: { cell: { variant: "datetime" } },
      },
      // 4. Account
      {
        id: "accountName",
        accessorKey: "accountId",
        header: t("activities.table.account"),
        size: 180,
        meta: { cell: { variant: "select", options: accountOptions } },
      },

      // === Identity / classification ===
      // 5. Type
      {
        accessorKey: "activityType",
        header: t("activities.type"),
        size: 150,
        enablePinning: false,
        meta: {
          cell: {
            variant: "select",
            options: activityTypeOptions,
            valueRenderer: (value: string, _option, rowData) => {
              const transaction = rowData as LocalTransaction | undefined;
              const subtype = transaction?.subtype;

              return (
                <ActivityTypeBadge
                  type={value as ActivityType}
                  subtype={shouldDisplaySubtype(transaction, value, subtype) ? subtype : undefined}
                  className="text-xs font-normal"
                />
              );
            },
          },
        },
      },
      // 6. Subtype (hidden by default; dynamic options based on activity type)
      {
        id: "subtype",
        accessorKey: "subtype",
        header: t("activityManager.form.subtype"),
        size: 160,
        enableSorting: false,
        enableHiding: true,
        meta: {
          cell: {
            variant: "select",
            // Dynamic options based on activity type
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            options: ((rowData: any) => {
              const activityType = rowData?.activityType?.toUpperCase();
              if (!activityType) return [];
              const allowedSubtypes = SUBTYPES_BY_ACTIVITY_TYPE[activityType] || [];
              return allowedSubtypes.map((subtype) => ({
                value: subtype,
                label: t(SUBTYPE_DISPLAY_NAMES[subtype], { defaultValue: subtype }),
              }));
            }) as any,
            allowEmpty: true,
            emptyLabel: t("activityManager.form.none"),
            valueRenderer: (value: string, option, rowData) => {
              const transaction = rowData as LocalTransaction | undefined;
              if (!shouldDisplaySubtype(transaction, transaction?.activityType, value)) {
                return null;
              }

              const displayLabel = getSubtypeDisplayLabel(value, option?.label);

              return (
                <Badge
                  variant="secondary"
                  className="min-w-0 max-w-full rounded-sm px-1.5 text-xs"
                  title={displayLabel}
                >
                  <span className="min-w-0 truncate">{displayLabel}</span>
                </Badge>
              );
            },
          },
        },
      },
      // 7. External (checkbox for TRANSFER_IN/TRANSFER_OUT only)
      {
        id: "isExternal",
        accessorKey: "isExternal",
        header: t("activities.grid.external"),
        size: 80,
        enableSorting: false,
        enableHiding: true,
        meta: {
          cell: {
            variant: "checkbox",
            // Only enabled for transfer types
            isDisabled: (rowData: unknown) => {
              const row = rowData as LocalTransaction;
              const activityType = row.activityType?.toUpperCase();
              return (
                activityType !== ActivityType.TRANSFER_IN &&
                activityType !== ActivityType.TRANSFER_OUT
              );
            },
          },
        },
      },
      // 8. Symbol
      {
        accessorKey: "assetSymbol",
        header: t("activities.table.symbol"),
        size: 160,
        meta: {
          cell: {
            variant: "symbol",
            isDisabled: (rowData: unknown) => {
              const row = rowData as LocalTransaction;
              return (
                isCashActivity(row.activityType ?? "") &&
                !isAssetBackedIncomeSubtype(row.activityType ?? "", row.subtype) &&
                !isTransferActivity(row.activityType)
              );
            },
            getDisplayContext: (rowData: unknown) => {
              const row = rowData as LocalTransaction;
              const symbol = (row.assetSymbol ?? "").trim().toUpperCase();
              if (!symbol || symbol === "CASH" || symbol.startsWith("$CASH")) {
                return undefined;
              }
              // Show contract description for options
              const parsed = row.instrumentType === "OPTION" ? parseOccSymbol(symbol) : null;
              if (parsed) {
                const expDisplay = new Date(parsed.expiration + "T12:00:00").toLocaleDateString(
                  "en-US",
                  { month: "short", day: "numeric" },
                );
                return `${expDisplay} $${parsed.strikePrice} ${parsed.optionType}`;
              }
              return getExchangeDisplayName(row.exchangeMic);
            },
            isClearable: (rowData: unknown) => {
              const row = rowData as LocalTransaction;
              return !isAssetIdentityRequired(row.activityType ?? "", row.subtype);
            },
            onSearch: handleSymbolSearch,
            onSelect: onSymbolSelect
              ? (rowIndex: number, _symbol: string, result?: SymbolSearchResult) => {
                  if (result) {
                    onSymbolSelect(rowIndex, result);
                  }
                }
              : undefined,
            onCreateCustomAsset,
          },
        },
      },

      // 9. Instrument Type (hidden by default, editable select)
      {
        id: "instrumentType",
        accessorKey: "instrumentType",
        header: t("activities.instrument"),
        size: 120,
        enableSorting: false,
        enableHiding: true,
        meta: {
          cell: {
            variant: "select",
            options: INSTRUMENT_TYPE_OPTIONS.map((opt) => ({
              value: opt.value,
              label: t(opt.labelKey),
            })),
            allowEmpty: true,
            emptyLabel: t("activities.grid.auto"),
          },
        },
      },

      // === Numbers (grouped, right-aligned) ===
      // 10. Quantity
      {
        accessorKey: "quantity",
        header: t("activities.table.quantity"),
        size: 120,
        enableSorting: false,
        meta: { cell: { variant: "number", step: 0.000001, valueType: "string" } },
      },
      // 9. Price
      {
        accessorKey: "unitPrice",
        header: t("activityManager.form.price"),
        size: 120,
        enableSorting: false,
        meta: {
          helpText: t("activities.grid.unitPriceHelp"),
          cell: { variant: "number", step: 0.000001, valueType: "string" },
        },
      },
      // 10. Amount (most important money column)
      {
        accessorKey: "amount",
        header: t("activityManager.form.totalCredit"),
        size: 120,
        enableSorting: false,
        meta: { cell: { variant: "number", step: 0.000001, valueType: "string" } },
      },
      // 11. Currency
      {
        accessorKey: "currency",
        header: t("activities.table.currency"),
        size: 110,
        enableSorting: false,
        meta: { cell: { variant: "currency" } },
      },
      // 12. Fee
      {
        accessorKey: "fee",
        header: t("activities.table.fee"),
        size: 100,
        enableSorting: false,
        meta: { cell: { variant: "number", step: 0.000001, valueType: "string" } },
      },
      // 13. FX Rate (lowest priority; often hidden)
      {
        accessorKey: "fxRate",
        header: t("activityManager.form.fxRate"),
        size: 100,
        enableSorting: false,
        meta: { cell: { variant: "number", step: 0.000001, valueType: "string" } },
      },

      // === Notes + actions ===
      // 14. Comment
      {
        accessorKey: "comment",
        header: t("activityManager.form.notes"),
        size: 260,
        enableSorting: false,
        meta: { cell: { variant: "long-text" } },
      },
      // 15. Activity Status (badge)
      {
        id: "activityStatus",
        accessorKey: "status",
        header: t("activities.status.title"),
        size: 100,
        enableSorting: false,
        enableHiding: true,
        cell: ({ row }) => {
          const status = row.original.status;
          if (!status) return <span className="text-muted-foreground">—</span>;
          const displayInfo = STATUS_DISPLAY[status] || {
            labelKey: status,
            variant: "default" as const,
          };
          return (
            <Badge variant={displayInfo.variant} className="text-xs font-normal">
              {t(displayInfo.labelKey, { defaultValue: status })}
            </Badge>
          );
        },
      },
      // 16. Actions
      {
        id: "actions",
        header: () => null,
        size: 64,
        enableSorting: false,
        enableResizing: false,
        enableHiding: false,
        cell: ({ row }) => (
          <div className="flex size-full items-center justify-center">
            <ActivityOperations
              activity={row.original}
              onEdit={onEditActivity}
              onDuplicate={onDuplicate}
              onDelete={onDelete}
              onLinkTransfer={onLinkTransfer}
              onUnlinkTransfer={onUnlinkTransfer}
            />
          </div>
        ),
      },
    ],
    [
      accountOptions,
      activityTypeOptions,
      handleSymbolSearch,
      onCreateCustomAsset,
      onDelete,
      onDuplicate,
      onEditActivity,
      onLinkTransfer,
      onUnlinkTransfer,
      onSymbolSelect,
      t,
    ],
  );

  return columns;
}
