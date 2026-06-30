use async_trait::async_trait;

use crate::errors::Result;

use super::models::ThemeMapping;

#[async_trait]
pub trait ThemeMappingProvider: Send + Sync {
    async fn theme_mappings(&self) -> Result<Vec<ThemeMapping>>;
}
