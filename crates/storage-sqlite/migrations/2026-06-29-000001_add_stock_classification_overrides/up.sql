CREATE TABLE IF NOT EXISTS stock_classification_overrides (
  stock_key TEXT PRIMARY KEY NOT NULL,
  asset_code TEXT,
  asset_name TEXT NOT NULL,
  sector TEXT,
  industry TEXT,
  theme_tags_json TEXT NOT NULL DEFAULT '[]',
  source TEXT NOT NULL DEFAULT 'manual',
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_stock_classification_overrides_asset_code
  ON stock_classification_overrides(asset_code);
