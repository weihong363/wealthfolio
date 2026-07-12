//! Market Intelligence data layer.
//!
//! This module intentionally contains objective market data models and
//! provider/repository traits only. AI inference and evidence ingestion are
//! separate future layers.

pub mod capital_flow;
pub mod eastmoney;
pub mod evidence;
pub mod flow_signal;
pub mod intraday;
pub mod macro_capital;
pub mod market_overview;
pub mod portfolio_exposure;
pub mod repository;
pub mod scheduler;
pub mod sector_rotation;
pub mod service;
pub mod theme_rotation;

pub use capital_flow::{
    CapitalFlowProvider, CapitalFlowRepository, CapitalFlowService, CapitalFlowSnapshot,
};
pub use eastmoney::{default_theme_mappings, EastmoneyMarketIntelligenceProvider};
pub use flow_signal::{
    flow_state, FlowSignal, FlowSignalInputs, MarketFlowSignalService, MarketLiquidityMetrics,
};
pub use intraday::{
    IntradayMarketIntelligenceProvider, MarketIntelligenceIntradayRepository,
    MarketIntelligenceIntradayService, MarketIntelligenceIntradaySnapshot,
};
pub use macro_capital::{
    MacroCapitalProvider, MacroCapitalRepository, MacroCapitalService, MacroCapitalSnapshot,
};
pub use market_overview::{
    MarketDataProvider, MarketOverviewService, MarketSnapshot, MarketSnapshotRepository,
};
pub use portfolio_exposure::{
    FundThemeExposureInput, PortfolioExposureInputProvider, PortfolioExposureService,
    PortfolioThemeExposure, PortfolioThemeExposureRepository,
};
pub use sector_rotation::{
    SectorRotationProvider, SectorRotationRepository, SectorRotationService, SectorRotationSnapshot,
};
pub use service::{
    regime, MarketIntelligenceService, MarketIntelligenceSummary, MarketRegimeAssessment,
    MarketRegimeMetrics,
};
pub use theme_rotation::{
    ThemeMapping, ThemeMappingProvider, ThemeRotationRepository, ThemeRotationRules,
    ThemeRotationService, ThemeRotationSnapshot,
};
