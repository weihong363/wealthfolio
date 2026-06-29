UPDATE assets
SET
  instrument_type = 'EQUITY',
  updated_at = CURRENT_TIMESTAMP
WHERE instrument_type = 'FUND'
  AND (
    CASE
      WHEN json_valid(provider_config)
      THEN json_extract(provider_config, '$.preferred_provider')
      ELSE NULL
    END = 'EASTMONEY_FUND'
    OR (
      quote_ccy = 'CNY'
      AND length(coalesce(display_code, instrument_symbol, '')) = 6
      AND coalesce(display_code, instrument_symbol, '') GLOB '[0-9][0-9][0-9][0-9][0-9][0-9]'
      AND CASE
        WHEN json_valid(metadata)
        THEN upper(coalesce(
          json_extract(metadata, '$.asset_type'),
          json_extract(metadata, '$.fundType'),
          ''
        ))
        ELSE ''
      END IN ('FUND', 'MUTUALFUND', 'MUTUAL_FUND', 'MUTUAL FUND', 'CN_FUND', 'CHINA_FUND')
    )
  );
