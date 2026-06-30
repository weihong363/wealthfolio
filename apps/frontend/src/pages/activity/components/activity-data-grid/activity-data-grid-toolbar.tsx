import {
  Button,
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
  Icons,
} from "@wealthfolio/ui";
import type { Table } from "@tanstack/react-table";
import { useTranslation } from "react-i18next";
import type { ChangesSummary, LocalTransaction } from "./types";

// Columns that can be toggled (exclude select, status indicator, actions)
const TOGGLEABLE_COLUMNS = [
  "activityType",
  "subtype",
  "instrumentType",
  "activityStatus",
  "date",
  "assetSymbol",
  "quantity",
  "unitPrice",
  "amount",
  "fee",
  "fxRate",
  "accountName",
  "currency",
  "comment",
];

interface ActivityDataGridToolbarProps {
  /** Number of rows currently selected */
  selectedRowCount: number;
  /** Number of selected rows that are pending review (synced/draft) */
  selectedPendingCount: number;
  /** Whether there are unsaved changes */
  hasUnsavedChanges: boolean;
  /** Summary of pending changes */
  changesSummary: ChangesSummary;
  /** Whether a save operation is in progress */
  isSaving: boolean;
  /** Table instance for column visibility */
  table: Table<LocalTransaction>;
  /** Handler for adding a new row */
  onAddRow: () => void;
  /** Handler for deleting selected rows */
  onDeleteSelected: () => void;
  /** Handler for approving selected synced activities */
  onApproveSelected: () => void;
  /** Handler for saving changes */
  onSave: () => void;
  /** Handler for canceling/discarding changes */
  onCancel: () => void;
  /** Handler invoked when user clicks "Link as internal pair" */
  onLinkSelected?: () => void;
  /** Whether the current selection is a valid TRANSFER_IN/TRANSFER_OUT pair */
  canLinkSelected?: boolean;
  /** Reason the current selection cannot be linked (used as button tooltip) */
  linkDisabledReason?: string;
  /** Whether a link operation is in progress */
  isLinking?: boolean;
  /** Handler invoked when user clicks "Unlink internal pair" */
  onUnlinkSelected?: () => void;
  /** Whether to show the unlink action for the current selection */
  showUnlinkSelected?: boolean;
  /** Whether the current selection is a linked TRANSFER_IN/TRANSFER_OUT pair */
  canUnlinkSelected?: boolean;
  /** Reason the current selection cannot be unlinked (used as button tooltip) */
  unlinkDisabledReason?: string;
  /** Whether an unlink operation is in progress */
  isUnlinking?: boolean;
}

/**
 * Toolbar component for the activity data grid
 * Displays status, selection info, and action buttons
 */
export function ActivityDataGridToolbar({
  selectedRowCount,
  selectedPendingCount,
  hasUnsavedChanges,
  changesSummary,
  isSaving,
  table,
  onAddRow,
  onDeleteSelected,
  onApproveSelected,
  onSave,
  onCancel,
  onLinkSelected,
  canLinkSelected,
  linkDisabledReason,
  isLinking,
  onUnlinkSelected,
  showUnlinkSelected,
  canUnlinkSelected,
  unlinkDisabledReason,
  isUnlinking,
}: ActivityDataGridToolbarProps) {
  const { t } = useTranslation();
  const columnDisplayNames: Record<string, string> = {
    activityType: t("activities.type"),
    subtype: t("activityManager.form.subtype"),
    activityStatus: t("activities.status.title"),
    date: t("activities.grid.dateTime"),
    assetSymbol: t("activities.table.symbol"),
    quantity: t("activities.table.quantity"),
    unitPrice: t("activityManager.form.price"),
    amount: t("activityManager.form.totalCredit"),
    fee: t("activities.table.fee"),
    fxRate: t("activityManager.form.fxRate"),
    accountName: t("activities.table.account"),
    currency: t("activities.table.currency"),
    instrumentType: t("activities.instrument"),
    comment: t("activityManager.form.notes"),
  };

  // Prevent mousedown from bubbling to document, which would clear DataGrid selection
  const handleMouseDown = (e: React.MouseEvent) => {
    e.stopPropagation();
  };

  return (
    <div
      className="bg-muted/20 flex flex-wrap items-center justify-between gap-2 rounded-md border px-2.5 py-1.5"
      onMouseDown={handleMouseDown}
    >
      <div className="text-muted-foreground flex items-center gap-2.5 text-xs">
        {/* Selection info */}
        {selectedRowCount > 0 && (
          <span className="font-medium">
            {selectedRowCount} row{selectedRowCount === 1 ? "" : "s"} selected
          </span>
        )}

        {/* Pending changes info */}
        {hasUnsavedChanges && (
          <div className="flex items-center gap-2">
            <span className="text-primary font-medium">
              {changesSummary.totalPendingChanges} pending change
              {changesSummary.totalPendingChanges === 1 ? "" : "s"}
            </span>
            <div className="bg-border h-3.5 w-px" />
            <div className="flex items-center gap-4">
              {changesSummary.newCount > 0 && (
                <span className="text-success flex items-center gap-1">
                  <Icons.PlusCircle className="h-3 w-3" />
                  <span className="font-medium">{changesSummary.newCount}</span>
                </span>
              )}
              {changesSummary.updatedCount > 0 && (
                <span className="flex items-center gap-1 text-blue-500 dark:text-blue-400">
                  <Icons.Pencil className="h-3 w-3" />
                  <span className="font-medium">{changesSummary.updatedCount}</span>
                </span>
              )}
              {changesSummary.deletedCount > 0 && (
                <span className="text-destructive flex items-center gap-1">
                  <Icons.Trash className="h-3 w-3" />
                  <span className="font-medium">{changesSummary.deletedCount}</span>
                </span>
              )}
            </div>
          </div>
        )}
      </div>

      {/* Action buttons */}
      <div className="flex items-center gap-1">
        <Button
          onClick={onAddRow}
          variant="outline"
          size="xs"
          className="shrink-0 rounded-md"
          title={t("activities.addTransaction")}
          aria-label={t("activities.addTransaction")}
        >
          <Icons.Plus className="h-3.5 w-3.5" />
          <span>{t("common.add")}</span>
        </Button>

        {/* Column visibility dropdown */}
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              variant="outline"
              size="xs"
              className="shrink-0 rounded-md px-2"
              title={t("activities.table.toggleColumns")}
              aria-label={t("activities.table.toggleColumns")}
            >
              <Icons.Settings2 className="h-3.5 w-3.5" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-44">
            <DropdownMenuLabel className="text-xs">
              {t("activities.table.toggleColumns")}
            </DropdownMenuLabel>
            <DropdownMenuSeparator />
            {table
              .getAllColumns()
              .filter((column) => TOGGLEABLE_COLUMNS.includes(column.id))
              .map((column) => (
                <DropdownMenuCheckboxItem
                  key={column.id}
                  className="text-xs"
                  checked={column.getIsVisible()}
                  onCheckedChange={(value) => column.toggleVisibility(!!value)}
                >
                  {columnDisplayNames[column.id] || column.id}
                </DropdownMenuCheckboxItem>
              ))}
          </DropdownMenuContent>
        </DropdownMenu>

        {selectedRowCount > 0 && (
          <>
            <div className="bg-border mx-1 h-4 w-px" />
            {selectedPendingCount > 0 && (
              <Button
                onClick={onApproveSelected}
                size="xs"
                variant="outline"
                className="shrink-0 rounded-md border-green-200 bg-green-50 text-xs text-green-700 hover:bg-green-100 hover:text-green-800 dark:border-green-800 dark:bg-green-900/20 dark:text-green-400 dark:hover:bg-green-900/40"
                title={t("activities.grid.approveSelected")}
                aria-label={t("activities.grid.approveSelected")}
                disabled={isSaving}
              >
                <Icons.CheckCircle className="h-3.5 w-3.5" />
                <span>{t("activities.grid.approveCount", { count: selectedPendingCount })}</span>
              </Button>
            )}
            {selectedRowCount === 2 && showUnlinkSelected && onUnlinkSelected ? (
              <Button
                onClick={onUnlinkSelected}
                size="xs"
                variant="outline"
                className="shrink-0 rounded-md text-xs"
                title={canUnlinkSelected ? "Unlink internal transfer" : unlinkDisabledReason}
                aria-label={t("activities.grid.unlinkInternalTransfer")}
                disabled={!canUnlinkSelected || isUnlinking || isSaving}
              >
                {isUnlinking ? (
                  <Icons.Spinner className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <Icons.Unlink className="h-3.5 w-3.5" />
                )}
                <span>{t("activities.grid.unlink")}</span>
              </Button>
            ) : selectedRowCount === 2 && onLinkSelected ? (
              <Button
                onClick={onLinkSelected}
                size="xs"
                variant="outline"
                className="shrink-0 rounded-md text-xs"
                title={canLinkSelected ? "Link as internal transfer" : linkDisabledReason}
                aria-label={t("activities.grid.linkInternalTransfer")}
                disabled={!canLinkSelected || isLinking || isSaving}
              >
                {isLinking ? (
                  <Icons.Spinner className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <Icons.Link className="h-3.5 w-3.5" />
                )}
                <span>{t("activities.grid.link")}</span>
              </Button>
            ) : null}
            <Button
              onClick={onDeleteSelected}
              size="xs"
              variant="destructive"
              className="shrink-0 rounded-md text-xs"
              title={t("activities.grid.deleteSelected")}
              aria-label={t("activities.grid.deleteSelected")}
              disabled={isSaving}
            >
              <Icons.Trash className="h-3.5 w-3.5" />
              <span>{t("common.delete")}</span>
            </Button>
          </>
        )}

        {hasUnsavedChanges && (
          <>
            <div className="bg-border mx-1 h-4 w-px" />
            <Button
              onClick={onSave}
              size="xs"
              className="shrink-0 rounded-md text-xs"
              title={t("activities.grid.saveChanges")}
              aria-label={t("activities.grid.saveChanges")}
              disabled={isSaving}
            >
              {isSaving ? (
                <Icons.Spinner className="h-3.5 w-3.5 animate-spin" />
              ) : (
                <Icons.Save className="h-3.5 w-3.5" />
              )}
              <span>{t("common.save")}</span>
            </Button>

            <Button
              onClick={onCancel}
              size="xs"
              variant="outline"
              className="shrink-0 rounded-md text-xs"
              title={t("activities.grid.discardChanges")}
              aria-label={t("activities.grid.discardChanges")}
              disabled={isSaving}
            >
              <Icons.Undo className="h-3.5 w-3.5" />
              <span>{t("common.cancel")}</span>
            </Button>
          </>
        )}
      </div>
    </div>
  );
}
