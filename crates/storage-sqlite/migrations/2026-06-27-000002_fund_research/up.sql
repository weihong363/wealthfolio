CREATE TABLE IF NOT EXISTS fund_research_snapshots (
    id TEXT PRIMARY KEY NOT NULL,
    fund_code TEXT NOT NULL,
    fund_name TEXT NOT NULL,
    snapshot_date TEXT NOT NULL,
    source TEXT NOT NULL,
    quote_json TEXT,
    nav_history_json TEXT NOT NULL DEFAULT '[]',
    performance_json TEXT,
    ai_features_json TEXT NOT NULL DEFAULT '{}',
    snapshot_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(fund_code, snapshot_date, source)
);

CREATE INDEX IF NOT EXISTS idx_fund_research_snapshots_fund_date
    ON fund_research_snapshots(fund_code, snapshot_date DESC);

CREATE TABLE IF NOT EXISTS fund_internal_holdings (
    id TEXT PRIMARY KEY NOT NULL,
    fund_code TEXT NOT NULL,
    report_date TEXT NOT NULL,
    rank INTEGER,
    asset_code TEXT,
    asset_name TEXT NOT NULL,
    asset_type TEXT NOT NULL,
    market TEXT,
    weight_pct REAL,
    theme_tags_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_fund_internal_holdings_fund_report
    ON fund_internal_holdings(fund_code, report_date DESC);

CREATE UNIQUE INDEX IF NOT EXISTS idx_fund_internal_holdings_unique_asset
    ON fund_internal_holdings(fund_code, report_date, asset_name, COALESCE(asset_code, ''));

CREATE TABLE IF NOT EXISTS fund_holding_changes (
    id TEXT PRIMARY KEY NOT NULL,
    fund_code TEXT NOT NULL,
    report_date TEXT NOT NULL,
    asset_code TEXT,
    asset_name TEXT NOT NULL,
    previous_weight_pct REAL,
    current_weight_pct REAL,
    weight_change_pct REAL,
    previous_rank INTEGER,
    current_rank INTEGER,
    change_type TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_fund_holding_changes_fund_report
    ON fund_holding_changes(fund_code, report_date DESC);

CREATE TABLE IF NOT EXISTS fund_sector_allocations (
    id TEXT PRIMARY KEY NOT NULL,
    fund_code TEXT NOT NULL,
    snapshot_date TEXT NOT NULL,
    sector TEXT NOT NULL,
    weight_pct REAL,
    previous_weight_pct REAL,
    weight_change_pct REAL,
    created_at TEXT NOT NULL,
    UNIQUE(fund_code, snapshot_date, sector)
);

CREATE TABLE IF NOT EXISTS fund_theme_exposures (
    id TEXT PRIMARY KEY NOT NULL,
    fund_code TEXT NOT NULL,
    snapshot_date TEXT NOT NULL,
    theme TEXT NOT NULL,
    weight_pct REAL NOT NULL,
    source TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(fund_code, snapshot_date, theme, source)
);

CREATE INDEX IF NOT EXISTS idx_fund_theme_exposures_theme_date
    ON fund_theme_exposures(theme, snapshot_date DESC);

CREATE TABLE IF NOT EXISTS fund_asset_allocations (
    id TEXT PRIMARY KEY NOT NULL,
    fund_code TEXT NOT NULL,
    snapshot_date TEXT NOT NULL,
    asset_class TEXT NOT NULL,
    weight_pct REAL,
    created_at TEXT NOT NULL,
    UNIQUE(fund_code, snapshot_date, asset_class)
);

CREATE TABLE IF NOT EXISTS fund_region_allocations (
    id TEXT PRIMARY KEY NOT NULL,
    fund_code TEXT NOT NULL,
    snapshot_date TEXT NOT NULL,
    region TEXT NOT NULL,
    weight_pct REAL,
    created_at TEXT NOT NULL,
    UNIQUE(fund_code, snapshot_date, region)
);

CREATE TABLE IF NOT EXISTS fund_risk_metrics (
    id TEXT PRIMARY KEY NOT NULL,
    fund_code TEXT NOT NULL,
    snapshot_date TEXT NOT NULL,
    max_drawdown_1m_pct REAL,
    max_drawdown_1y_pct REAL,
    annualized_volatility_pct REAL,
    sharpe_ratio REAL,
    created_at TEXT NOT NULL,
    UNIQUE(fund_code, snapshot_date)
);

CREATE TABLE IF NOT EXISTS fund_announcements (
    id TEXT PRIMARY KEY NOT NULL,
    fund_code TEXT NOT NULL,
    announcement_date TEXT,
    title TEXT NOT NULL,
    url TEXT,
    category TEXT,
    importance TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_fund_announcements_fund_date
    ON fund_announcements(fund_code, announcement_date DESC);

CREATE TABLE IF NOT EXISTS sector_rotation_signals (
    id TEXT PRIMARY KEY NOT NULL,
    theme TEXT NOT NULL,
    signal_date TEXT NOT NULL,
    momentum_1m REAL,
    momentum_3m REAL,
    inflow_score REAL,
    crowding_score REAL,
    drawdown_risk REAL,
    signal_strength TEXT NOT NULL,
    reason_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    UNIQUE(theme, signal_date)
);

CREATE INDEX IF NOT EXISTS idx_sector_rotation_signals_theme_date
    ON sector_rotation_signals(theme, signal_date DESC);

CREATE TABLE IF NOT EXISTS rebalance_alerts (
    id TEXT PRIMARY KEY NOT NULL,
    fund_code TEXT NOT NULL,
    alert_date TEXT NOT NULL,
    alert_type TEXT NOT NULL,
    severity TEXT NOT NULL,
    title TEXT NOT NULL,
    message TEXT NOT NULL,
    evidence_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    read_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_rebalance_alerts_fund_read
    ON rebalance_alerts(fund_code, read_at, alert_date DESC);
