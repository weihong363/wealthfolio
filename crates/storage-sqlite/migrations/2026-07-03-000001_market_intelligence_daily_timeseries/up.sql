ALTER TABLE capital_flow_snapshots
    ADD COLUMN granularity TEXT NOT NULL DEFAULT 'daily';

ALTER TABLE sector_rotation_snapshots
    ADD COLUMN granularity TEXT NOT NULL DEFAULT 'daily';

ALTER TABLE theme_rotation_snapshots
    ADD COLUMN source TEXT NOT NULL DEFAULT 'sector_rotation_rules';

ALTER TABLE theme_rotation_snapshots
    ADD COLUMN granularity TEXT NOT NULL DEFAULT 'daily';

CREATE INDEX IF NOT EXISTS idx_capital_flow_snapshots_date
    ON capital_flow_snapshots(date DESC);

CREATE INDEX IF NOT EXISTS idx_sector_rotation_snapshots_date_ranking
    ON sector_rotation_snapshots(date DESC, ranking ASC);

CREATE INDEX IF NOT EXISTS idx_theme_rotation_snapshots_date_ranking
    ON theme_rotation_snapshots(date DESC, ranking ASC);

CREATE TABLE IF NOT EXISTS market_intelligence_intraday_snapshots (
    id TEXT PRIMARY KEY NOT NULL,
    market TEXT NOT NULL,
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    open REAL,
    close REAL,
    high REAL,
    low REAL,
    volume REAL,
    amount REAL,
    net_flow REAL,
    change_pct REAL,
    turnover REAL,
    ranking INTEGER,
    source TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_market_intelligence_intraday_kind_name_time
    ON market_intelligence_intraday_snapshots(kind, name, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_market_intelligence_intraday_time
    ON market_intelligence_intraday_snapshots(timestamp DESC);
