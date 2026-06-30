CREATE TABLE IF NOT EXISTS market_snapshots (
    id TEXT PRIMARY KEY NOT NULL,
    market TEXT NOT NULL,
    index_name TEXT NOT NULL,
    price REAL NOT NULL,
    change_pct REAL NOT NULL,
    turnover REAL,
    timestamp TEXT NOT NULL,
    source TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_market_snapshots_market_index_time
    ON market_snapshots(market, index_name, timestamp DESC);

CREATE TABLE IF NOT EXISTS capital_flow_snapshots (
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

CREATE INDEX IF NOT EXISTS idx_capital_flow_snapshots_market_category_date
    ON capital_flow_snapshots(market, category, date DESC);

CREATE TABLE IF NOT EXISTS sector_rotation_snapshots (
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

CREATE INDEX IF NOT EXISTS idx_sector_rotation_snapshots_market_sector_date
    ON sector_rotation_snapshots(market, sector, date DESC);

CREATE TABLE IF NOT EXISTS theme_rotation_snapshots (
    id TEXT PRIMARY KEY NOT NULL,
    theme TEXT NOT NULL,
    date TEXT NOT NULL,
    flow_score REAL,
    momentum REAL,
    ranking INTEGER,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_theme_rotation_snapshots_theme_date
    ON theme_rotation_snapshots(theme, date DESC);

CREATE TABLE IF NOT EXISTS portfolio_theme_exposures (
    id TEXT PRIMARY KEY NOT NULL,
    portfolio_id TEXT NOT NULL,
    theme TEXT NOT NULL,
    weight_pct REAL NOT NULL,
    market_value REAL NOT NULL,
    source TEXT NOT NULL,
    timestamp TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_portfolio_theme_exposures_portfolio_time
    ON portfolio_theme_exposures(portfolio_id, timestamp DESC);
