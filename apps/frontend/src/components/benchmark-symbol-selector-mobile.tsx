import { searchTicker } from "@/adapters";
import { Button } from "@wealthfolio/ui/components/ui/button";
import { Icons } from "@wealthfolio/ui/components/ui/icons";
import { ScrollArea } from "@wealthfolio/ui/components/ui/scroll-area";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "@wealthfolio/ui/components/ui/sheet";
import { Skeleton } from "@wealthfolio/ui/components/ui/skeleton";
import { SymbolSearchResult } from "@/lib/types";
import { getExchangeDisplayName } from "@/lib/constants";

import { cn } from "@/lib/utils";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";

// Predefined benchmarks — group names & descriptions are i18n keys resolved at render time
// exchangeMic is undefined for indices (will use "INDEX" as pseudo-MIC)
// exchangeMic is set for ETFs that trade on real exchanges
const BENCHMARKS = [
  {
    groupKey: "benchmark.groups.usMarket",
    items: [
      { symbol: "^GSPC", name: "S&P 500", descKey: "benchmark.descriptions.sp500" },
      { symbol: "^NDX", name: "Nasdaq 100", descKey: "benchmark.descriptions.nasdaq100" },
      { symbol: "^RUT", name: "Russell 2000", descKey: "benchmark.descriptions.russell2000" },
      { symbol: "^DJI", name: "Dow Jones", descKey: "benchmark.descriptions.dowJones" },
    ],
  },
  {
    groupKey: "benchmark.groups.european",
    items: [
      { symbol: "^FTSE", name: "FTSE 100", descKey: "benchmark.descriptions.ftse100" },
      { symbol: "^STOXX50E", name: "EURO STOXX 50", descKey: "benchmark.descriptions.euroStoxx50" },
      { symbol: "^GDAXI", name: "DAX", descKey: "benchmark.descriptions.dax" },
      { symbol: "^FCHI", name: "CAC 40", descKey: "benchmark.descriptions.cac40" },
      { symbol: "^IBEX", name: "IBEX 35", descKey: "benchmark.descriptions.ibex35" },
      { symbol: "^AEX", name: "AEX", descKey: "benchmark.descriptions.aex" },
      { symbol: "^OMX", name: "OMX Stockholm 30", descKey: "benchmark.descriptions.omxStockholm30" },
    ],
  },
  {
    groupKey: "benchmark.groups.asian",
    items: [
      { symbol: "^N225", name: "Nikkei 225", descKey: "benchmark.descriptions.nikkei225" },
      { symbol: "^HSI", name: "Hang Seng", descKey: "benchmark.descriptions.hangSeng" },
      { symbol: "000001.SS", name: "Shanghai Composite", descKey: "benchmark.descriptions.shanghaiComposite" },
      { symbol: "^KS11", name: "KOSPI", descKey: "benchmark.descriptions.kospi" },
      { symbol: "^TWII", name: "Taiwan Weighted", descKey: "benchmark.descriptions.taiwanWeighted" },
      { symbol: "^AXJO", name: "ASX 200", descKey: "benchmark.descriptions.asx200" },
      { symbol: "^BSESN", name: "BSE Sensex", descKey: "benchmark.descriptions.bseSensex" },
      { symbol: "^NSEI", name: "NIFTY 50", descKey: "benchmark.descriptions.nifty50" },
    ],
  },
  {
    groupKey: "benchmark.groups.globalEmerging",
    items: [
      { symbol: "EEM", name: "MSCI Emerging Markets", descKey: "benchmark.descriptions.msciEmerging", exchangeMic: "ARCX" },
      { symbol: "ACWI", name: "MSCI All Country World", descKey: "benchmark.descriptions.msciAcwi", exchangeMic: "XNAS" },
      { symbol: "IEFA", name: "Core MSCI EAFE", descKey: "benchmark.descriptions.msciEafe", exchangeMic: "ARCX" },
    ],
  },
  {
    groupKey: "benchmark.groups.etfs",
    items: [
      { symbol: "VOO", name: "Vanguard S&P 500", descKey: "benchmark.descriptions.voo", exchangeMic: "ARCX" },
      { symbol: "VTI", name: "Vanguard Total Stock", descKey: "benchmark.descriptions.vti", exchangeMic: "ARCX" },
      { symbol: "VEA", name: "Vanguard FTSE Developed", descKey: "benchmark.descriptions.vea", exchangeMic: "ARCX" },
      { symbol: "VWO", name: "Vanguard FTSE Emerging", descKey: "benchmark.descriptions.vwo", exchangeMic: "ARCX" },
    ],
  },
];

interface BenchmarkSymbolSelectorMobileProps {
  onSelect: (symbol: { id: string; name: string }) => void;
  className?: string;
  iconOnly?: boolean;
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
}

export function BenchmarkSymbolSelectorMobile({
  onSelect,
  className,
  iconOnly = false,
  open: controlledOpen,
  onOpenChange,
}: BenchmarkSymbolSelectorMobileProps) {
  const { t } = useTranslation();
  const [internalOpen, setInternalOpen] = useState(false);
  const open = controlledOpen !== undefined ? controlledOpen : internalOpen;
  const setOpen = onOpenChange !== undefined ? onOpenChange : setInternalOpen;

  const [searchQuery, setSearchQuery] = useState("");

  // Query for dynamic ticker search
  const {
    data: searchResults,
    isLoading,
    isError,
  } = useQuery<SymbolSearchResult[], Error>({
    queryKey: ["benchmark-ticker-search", searchQuery],
    queryFn: () => searchTicker(searchQuery),
    enabled: searchQuery?.length > 2,
  });

  // Sort search results by score if available
  const sortedSearchResults = searchResults?.sort((a, b) => b.score - a.score) ?? [];

  // Filter out search results that are already in predefined benchmarks
  const existingSymbols = BENCHMARKS.flatMap((group) => group.items.map((item) => item.symbol));
  const filteredSearchResults = sortedSearchResults.filter(
    (result) => !existingSymbols.includes(result.symbol),
  );

  const handleBenchmarkSelect = (benchmark: {
    symbol: string;
    name: string;
    exchangeMic?: string;
  }) => {
    onSelect({ id: benchmark.symbol, name: benchmark.name });
    setOpen(false);
    setSearchQuery("");
  };

  const handleSearchResultSelect = (ticker: SymbolSearchResult) => {
    onSelect({
      id: ticker.existingAssetId || ticker.symbol,
      name: ticker.longName || ticker.symbol,
    });
    setOpen(false);
    setSearchQuery("");
  };

  return (
    <Sheet open={open} onOpenChange={setOpen}>
      <SheetTrigger asChild>
        <Button
          variant="outline"
          aria-label={iconOnly ? t("benchmark.addBenchmark") : undefined}
          className={cn(
            "bg-secondary/30 hover:bg-muted/80 flex items-center gap-1.5 rounded-md border-[1.5px] border-none text-sm font-medium",
            iconOnly ? "h-9 w-9 p-0" : "h-8 px-3 py-1",
            className,
          )}
          size={iconOnly ? "icon" : "sm"}
        >
          <Icons.TrendingUp className="h-4 w-4" />
          {!iconOnly && t("benchmark.addBenchmark")}
        </Button>
      </SheetTrigger>
      <SheetContent side="bottom" className="rounded-t-4xl mx-1 h-[85vh] p-0">
        <SheetHeader className="border-border border-b px-6 py-4">
          <SheetTitle>{t("benchmark.selectBenchmark")}</SheetTitle>
          <SheetDescription>{t("benchmark.chooseBenchmark")}</SheetDescription>
        </SheetHeader>

        <div className="flex h-[calc(85vh-5rem)] flex-col">
          {/* Search Input */}
          <div className="border-border border-b px-6 py-3">
            <div className="relative">
              <Icons.Search className="text-muted-foreground absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2" />
              <input
                type="text"
                placeholder={t("benchmark.searchBenchmarks")}
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="bg-background border-input ring-offset-background placeholder:text-muted-foreground focus-visible:ring-ring h-10 w-full rounded-md border px-3 py-2 pl-9 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2"
              />
            </div>
          </div>

          {/* Results */}
          <ScrollArea className="flex-1 px-6 py-4">
            {/* Loading state for search results */}
            {isLoading && searchQuery.length > 2 && (
              <div className="space-y-2">
                <div className="text-muted-foreground mb-3 text-sm font-medium">Searching...</div>
                <Skeleton className="h-16 w-full" />
                <Skeleton className="h-16 w-full" />
                <Skeleton className="h-16 w-full" />
              </div>
            )}

            {/* Error state for search results */}
            {isError && searchQuery.length > 2 && (
              <div className="text-muted-foreground py-8 text-center text-sm">
                Error searching for symbols. Please try again.
              </div>
            )}

            {/* Dynamic search results */}
            {!isLoading &&
              !isError &&
              filteredSearchResults.length > 0 &&
              searchQuery.length > 2 && (
                <div className="mb-6">
                  <h3 className="text-muted-foreground mb-3 text-sm font-medium">Search Results</h3>
                  <div className="space-y-2">
                    {filteredSearchResults.slice(0, 8).map((ticker) => (
                      <button
                        key={ticker.symbol}
                        onClick={() => handleSearchResultSelect(ticker)}
                        className="hover:bg-accent active:bg-accent/80 focus:border-primary flex w-full items-center gap-3 rounded-lg border border-transparent p-3 text-left transition-colors focus:outline-none"
                      >
                        <div className="bg-primary/10 flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-full">
                          <Icons.TrendingUp className="text-primary h-5 w-5" />
                        </div>
                        <div className="min-w-0 flex-1">
                          <div className="flex items-center gap-2">
                            <span className="text-foreground truncate font-medium">
                              {ticker.longName || ticker.symbol}
                            </span>
                            <span className="text-muted-foreground text-xs">{ticker.symbol}</span>
                          </div>
                          {ticker.exchange && (
                            <div className="text-muted-foreground text-sm">
                              {ticker.exchangeName || getExchangeDisplayName(ticker.exchange)}
                            </div>
                          )}
                        </div>
                        <Icons.ChevronRight className="text-muted-foreground h-5 w-5 flex-shrink-0" />
                      </button>
                    ))}
                  </div>
                </div>
              )}

            {/* Predefined benchmark groups */}
            {(searchQuery.length === 0 || !isLoading) && (
              <div className="space-y-6">
                {BENCHMARKS.map((group) => {
                  const filteredItems = group.items.filter(
                    (benchmark) =>
                      searchQuery.length === 0 ||
                      benchmark.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
                      benchmark.symbol.toLowerCase().includes(searchQuery.toLowerCase()) ||
                      benchmark.descKey.toLowerCase().includes(searchQuery.toLowerCase()),
                  );

                  if (filteredItems.length === 0) return null;

                  return (
                    <div key={group.groupKey}>
                      <h3 className="text-muted-foreground mb-3 text-sm font-medium">
                        {t(group.groupKey)}
                      </h3>
                      <div className="space-y-2">
                        {filteredItems.map((benchmark) => (
                          <button
                            key={benchmark.symbol}
                            onClick={() => handleBenchmarkSelect(benchmark)}
                            className="hover:bg-accent active:bg-accent/80 focus:border-primary flex w-full items-center gap-3 rounded-lg border border-transparent p-3 text-left transition-colors focus:outline-none"
                          >
                            <div className="bg-primary/10 flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-full">
                              <Icons.TrendingUp className="text-primary h-5 w-5" />
                            </div>
                            <div className="min-w-0 flex-1">
                              <div className="flex items-center gap-2">
                                <span className="text-foreground font-medium">
                                  {benchmark.name}
                                </span>
                                <span className="text-muted-foreground text-xs">
                                  {benchmark.symbol}
                                </span>
                              </div>
                              <div className="text-muted-foreground text-sm">
                                {t(benchmark.descKey)}
                              </div>
                            </div>
                            <Icons.ChevronRight className="text-muted-foreground h-5 w-5 flex-shrink-0" />
                          </button>
                        ))}
                      </div>
                    </div>
                  );
                })}
              </div>
            )}

            {/* Empty state */}
            {searchQuery.length > 0 &&
              !isLoading &&
              filteredSearchResults.length === 0 &&
              BENCHMARKS.every(
                (group) =>
                  group.items.filter(
                    (benchmark) =>
                      benchmark.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
                      benchmark.symbol.toLowerCase().includes(searchQuery.toLowerCase()) ||
                      benchmark.descKey.toLowerCase().includes(searchQuery.toLowerCase()),
                  ).length === 0,
              ) && (
                <div className="text-muted-foreground flex h-full items-center justify-center text-sm">
                  {t("benchmark.noBenchmarksFound")}
                </div>
              )}
          </ScrollArea>
        </div>
      </SheetContent>
    </Sheet>
  );
}
