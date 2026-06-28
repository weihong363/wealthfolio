import type { ImportValidationStatus, QuoteImport } from "@/lib/types/quote-import";
import { Badge } from "@wealthfolio/ui/components/ui/badge";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@wealthfolio/ui/components/ui/card";
import { Icons } from "@wealthfolio/ui/components/ui/icons";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@wealthfolio/ui/components/ui/table";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@wealthfolio/ui/components/ui/tooltip";
import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";

function formatValidationStatus(status: ImportValidationStatus, t: TFunction): string {
  switch (status) {
    case "valid":
      return t("marketData.statusValid");
    case "warning":
      return t("marketData.statusWarning");
    case "error":
      return t("marketData.statusError");
    default:
      return status;
  }
}

function getStatusVariant(status: ImportValidationStatus): "success" | "destructive" | "warning" {
  switch (status) {
    case "valid":
      return "success";
    case "warning":
      return "warning";
    case "error":
      return "destructive";
    default:
      return "destructive";
  }
}

interface QuotePreviewTableProps {
  quotes: QuoteImport[];
  maxRows?: number;
}

export function QuotePreviewTable({ quotes, maxRows = 10 }: QuotePreviewTableProps) {
  const { t } = useTranslation();
  const displayQuotes = quotes.slice(0, maxRows);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Icons.FileText className="h-5 w-5" />
          {t("marketData.previewDataWithRows", { count: quotes.length })}
        </CardTitle>
        <CardDescription>{t("marketData.reviewFirstRows", { count: maxRows })}</CardDescription>
      </CardHeader>
      <CardContent>
        <div className="rounded-md border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{t("marketData.symbol")}</TableHead>
                <TableHead>{t("marketData.date")}</TableHead>
                <TableHead>{t("marketData.open")}</TableHead>
                <TableHead>{t("marketData.high")}</TableHead>
                <TableHead>{t("marketData.low")}</TableHead>
                <TableHead>{t("marketData.close")}</TableHead>
                <TableHead>{t("marketData.volume")}</TableHead>
                <TableHead>{t("marketData.currency")}</TableHead>
                <TableHead>{t("marketData.status")}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              <TooltipProvider>
                {displayQuotes.map((quote, index) => {
                  const hasError = quote.validationStatus === "error";
                  const hasWarning = quote.validationStatus === "warning";
                  const errorMessage = quote.errorMessage;

                  return (
                    <TableRow
                      key={index}
                      className={
                        hasError
                          ? "bg-destructive/5 hover:bg-destructive/10"
                          : hasWarning
                            ? "bg-warning/5 hover:bg-warning/10"
                            : undefined
                      }
                    >
                      <TableCell className="font-medium">
                        <div className="flex items-center gap-1.5">
                          {errorMessage && (
                            <Tooltip>
                              <TooltipTrigger asChild>
                                <Icons.AlertCircle className="text-destructive h-4 w-4 shrink-0 cursor-help" />
                              </TooltipTrigger>
                              <TooltipContent
                                side="right"
                                className="bg-destructive text-destructive-foreground max-w-xs"
                              >
                                <p>{errorMessage}</p>
                              </TooltipContent>
                            </Tooltip>
                          )}
                          {quote.displaySymbol ?? quote.symbol}
                        </div>
                      </TableCell>
                      <TableCell>{quote.date}</TableCell>
                      <TableCell>{quote.open ?? "-"}</TableCell>
                      <TableCell>{quote.high ?? "-"}</TableCell>
                      <TableCell>{quote.low ?? "-"}</TableCell>
                      <TableCell className="font-medium">{quote.close}</TableCell>
                      <TableCell>{quote.volume ?? "-"}</TableCell>
                      <TableCell>{quote.currency}</TableCell>
                      <TableCell>
                        <Badge
                          variant={getStatusVariant(quote.validationStatus)}
                          className="whitespace-nowrap"
                        >
                          {formatValidationStatus(quote.validationStatus, t)}
                        </Badge>
                      </TableCell>
                    </TableRow>
                  );
                })}
              </TooltipProvider>
            </TableBody>
          </Table>
        </div>
        {quotes.length > maxRows && (
          <p className="text-muted-foreground mt-2 text-sm">
            {t("marketData.showingRows", { shown: maxRows, total: quotes.length })}
          </p>
        )}
      </CardContent>
    </Card>
  );
}
