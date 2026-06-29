import { NetworkError, RateLimitError } from "./errors";

const DEFAULT_TIMEOUT_MS = 15_000;
const MAX_RETRIES = 2;
const RETRY_BASE_DELAY_MS = 800;

export interface FetchOptions {
  referer?: string;
  timeoutMs?: number;
}

// Simple TTL cache
const cache = new Map<string, { data: unknown; expiresAt: number }>();

function cacheKey(url: string): string {
  return url;
}

export function cacheGet<T>(key: string): T | undefined {
  const entry = cache.get(key);
  if (!entry) return undefined;
  if (Date.now() > entry.expiresAt) {
    cache.delete(key);
    return undefined;
  }
  return entry.data as T;
}

export function cacheSet(key: string, data: unknown, ttlMs: number): void {
  cache.set(key, { data, expiresAt: Date.now() + ttlMs });
}

export function cacheInvalidate(pattern?: string): void {
  if (!pattern) {
    cache.clear();
    return;
  }
  for (const key of cache.keys()) {
    if (key.includes(pattern)) cache.delete(key);
  }
}

async function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

const PROXY_URL = "/api/v1/eastmoney/proxy";

/**
 * Fetch JSON from an Eastmoney API URL through the backend proxy (to avoid CORS).
 * Falls back to direct fetch in Tauri desktop mode (where CORS is not enforced).
 */
export async function fetchJson<T>(
  url: string,
  opts: FetchOptions = {},
  cacheTtlMs?: number,
): Promise<T> {
  const ck = cacheKey(url);
  if (cacheTtlMs !== undefined) {
    const cached = cacheGet<T>(ck);
    if (cached !== undefined) return cached;
  }

  let lastError: unknown;

  for (let attempt = 0; attempt <= MAX_RETRIES; attempt++) {
    if (attempt > 0) {
      await sleep(RETRY_BASE_DELAY_MS * Math.pow(2, attempt - 1));
    }

    try {
      const controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), opts.timeoutMs ?? DEFAULT_TIMEOUT_MS);

      // Route through backend proxy to avoid CORS issues in browser mode
      const proxyRes = await fetch(PROXY_URL, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ url, referer: opts.referer }),
        signal: controller.signal,
        credentials: "same-origin",
      });

      clearTimeout(timeout);

      if (!proxyRes.ok) {
        throw new NetworkError(url, `Proxy HTTP ${proxyRes.status}`);
      }

      const proxyData = (await proxyRes.json()) as {
        status: number;
        body: T;
      };

      if (proxyData.status === 429) throw new RateLimitError();
      if (proxyData.status >= 400) {
        throw new NetworkError(url, `Eastmoney HTTP ${proxyData.status}`);
      }

      const data = proxyData.body;

      if (cacheTtlMs !== undefined) {
        cacheSet(ck, data, cacheTtlMs);
      }

      return data;
    } catch (err) {
      lastError = err;
      if (err instanceof RateLimitError) throw err;
      if (err instanceof DOMException && err.name === "AbortError") {
        lastError = new NetworkError(url, "Request timeout");
      }
    }
  }

  throw lastError instanceof Error ? lastError : new NetworkError(url, String(lastError));
}
