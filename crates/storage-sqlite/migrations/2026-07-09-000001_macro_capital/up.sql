CREATE TABLE IF NOT EXISTS macro_capital_snapshots (
    id TEXT PRIMARY KEY NOT NULL,
    indicator TEXT NOT NULL,
    market TEXT NOT NULL,
    date TEXT NOT NULL,
    value REAL NOT NULL,
    change REAL,
    unit TEXT,
    source TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_macro_capital_snapshots_indicator_date
    ON macro_capital_snapshots(indicator, date DESC);

CREATE INDEX IF NOT EXISTS idx_macro_capital_snapshots_date
    ON macro_capital_snapshots(date DESC);
