INSERT OR IGNORE INTO market_data_providers (
    id,
    name,
    description,
    url,
    priority,
    enabled,
    logo_filename,
    last_synced_at,
    last_sync_status,
    last_sync_error,
    provider_type,
    config
)
VALUES (
    'EASTMONEY_STOCK',
    'Eastmoney Stock',
    'Eastmoney Stock provides stock profile, industry and concept metadata for China A/BJ, Hong Kong and US equities.',
    'https://quote.eastmoney.com/',
    11,
    TRUE,
    NULL,
    NULL,
    NULL,
    NULL,
    'builtin',
    '{"quote_base_url":"https://quote.eastmoney.com","api_base_url":"https://push2.eastmoney.com","timeout_secs":15}'
);
