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
    'EASTMONEY_FUND',
    'Eastmoney Fund',
    'Eastmoney Fund provides NAV and research data for mainland China mutual funds.',
    'https://fund.eastmoney.com/',
    10,
    TRUE,
    NULL,
    NULL,
    NULL,
    NULL,
    'builtin',
    '{"base_url":"https://fund.eastmoney.com","f10_base_url":"https://fundf10.eastmoney.com","timeout_secs":15}'
);
