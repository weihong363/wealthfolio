DROP INDEX IF EXISTS idx_market_intelligence_intraday_time;
DROP INDEX IF EXISTS idx_market_intelligence_intraday_kind_name_time;
DROP TABLE IF EXISTS market_intelligence_intraday_snapshots;

DROP INDEX IF EXISTS idx_theme_rotation_snapshots_date_ranking;
DROP INDEX IF EXISTS idx_sector_rotation_snapshots_date_ranking;
DROP INDEX IF EXISTS idx_capital_flow_snapshots_date;

CREATE TABLE theme_rotation_snapshots__old (
    id TEXT PRIMARY KEY NOT NULL,
    theme TEXT NOT NULL,
    date TEXT NOT NULL,
    flow_score REAL,
    momentum REAL,
    ranking INTEGER,
    created_at TEXT NOT NULL
);

INSERT INTO theme_rotation_snapshots__old
    (id, theme, date, flow_score, momentum, ranking, created_at)
SELECT id, theme, date, flow_score, momentum, ranking, created_at
FROM theme_rotation_snapshots;

DROP TABLE theme_rotation_snapshots;
ALTER TABLE theme_rotation_snapshots__old RENAME TO theme_rotation_snapshots;

CREATE INDEX IF NOT EXISTS idx_theme_rotation_snapshots_theme_date
    ON theme_rotation_snapshots(theme, date DESC);

CREATE TABLE sector_rotation_snapshots__old (
    id TEXT PRIMARY KEY NOT NULL,
    market TEXT NOT NULL,
    sector TEXT NOT NULL,
    date TEXT NOT NULL,
    net_flow REAL,
    change_pct REAL,
    turnover REAL,
    ranking INTEGER,
    source TEXT NOT NULL,
    created_at TEXT NOT NULL
);

INSERT INTO sector_rotation_snapshots__old
    (id, market, sector, date, net_flow, change_pct, turnover, ranking, source, created_at)
SELECT id, market, sector, date, net_flow, change_pct, turnover, ranking, source, created_at
FROM sector_rotation_snapshots;

DROP TABLE sector_rotation_snapshots;
ALTER TABLE sector_rotation_snapshots__old RENAME TO sector_rotation_snapshots;

CREATE INDEX IF NOT EXISTS idx_sector_rotation_snapshots_market_sector_date
    ON sector_rotation_snapshots(market, sector, date DESC);

CREATE TABLE capital_flow_snapshots__old (
    id TEXT PRIMARY KEY NOT NULL,
    market TEXT NOT NULL,
    date TEXT NOT NULL,
    category TEXT NOT NULL,
    inflow REAL,
    outflow REAL,
    net_flow REAL NOT NULL,
    source TEXT NOT NULL,
    created_at TEXT NOT NULL
);

INSERT INTO capital_flow_snapshots__old
    (id, market, date, category, inflow, outflow, net_flow, source, created_at)
SELECT id, market, date, category, inflow, outflow, net_flow, source, created_at
FROM capital_flow_snapshots;

DROP TABLE capital_flow_snapshots;
ALTER TABLE capital_flow_snapshots__old RENAME TO capital_flow_snapshots;

CREATE INDEX IF NOT EXISTS idx_capital_flow_snapshots_market_category_date
    ON capital_flow_snapshots(market, category, date DESC);
