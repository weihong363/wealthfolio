use std::sync::Arc;
use std::time::Duration as StdDuration;

use chrono::{Datelike, Timelike, Utc, Weekday};
use chrono_tz::Asia::Shanghai;
use log::{debug, info, warn};

use super::service::MarketIntelligenceService;

pub async fn run_periodic_market_intelligence_refresh(
    service: Arc<MarketIntelligenceService>,
    initial_delay: StdDuration,
    daily_interval: StdDuration,
    intraday_interval: StdDuration,
) {
    tokio::time::sleep(initial_delay).await;
    info!(
        "Market intelligence scheduler started (daily: {}h, intraday: {}m)",
        daily_interval.as_secs() / 3600,
        intraday_interval.as_secs() / 60
    );

    let daily_service = Arc::clone(&service);
    tokio::spawn(async move {
        loop {
            info!("Market intelligence daily refresh: starting");
            if let Err(error) = daily_service.refresh_market_data().await {
                warn!("Market intelligence daily refresh failed: {error}");
            }
            tokio::time::sleep(daily_interval).await;
        }
    });

    loop {
        if is_cn_hk_market_session() {
            match service.refresh_intraday_market_data().await {
                Ok(count) => {
                    debug!("Market intelligence intraday refresh saved {count} snapshots");
                }
                Err(error) => {
                    warn!("Market intelligence intraday refresh failed: {error}");
                }
            }
        }
        tokio::time::sleep(intraday_interval).await;
    }
}

fn is_cn_hk_market_session() -> bool {
    let now = Utc::now().with_timezone(&Shanghai);
    if matches!(now.weekday(), Weekday::Sat | Weekday::Sun) {
        return false;
    }
    let minutes = now.hour() * 60 + now.minute();
    let morning = (9 * 60 + 15)..=(11 * 60 + 45);
    let afternoon = (13 * 60)..=(15 * 60 + 15);
    morning.contains(&minutes) || afternoon.contains(&minutes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn market_session_function_is_callable() {
        let _ = is_cn_hk_market_session();
    }
}
