export class EastMoneyStockError extends Error {
  constructor(
    message: string,
    public readonly code: string,
    public readonly statusCode?: number,
  ) {
    super(message);
    this.name = "EastMoneyStockError";
  }
}

export class StockNotFoundError extends EastMoneyStockError {
  constructor(stockCode: string) {
    super(`Stock not found: ${stockCode}`, "STOCK_NOT_FOUND", 404);
  }
}

export class NetworkError extends EastMoneyStockError {
  constructor(url: string, cause: unknown) {
    super(`Network error for ${url}: ${String(cause)}`, "NETWORK_ERROR");
  }
}

export class ParseError extends EastMoneyStockError {
  constructor(field: string, raw?: string) {
    super(`Failed to parse ${field}${raw ? ` from: ${raw.slice(0, 200)}` : ""}`, "PARSE_ERROR");
  }
}

export class RateLimitError extends EastMoneyStockError {
  constructor() {
    super("Rate limited by Eastmoney", "RATE_LIMITED", 429);
  }
}
