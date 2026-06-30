use async_trait::async_trait;

use crate::errors::Result;

use super::models::EvidenceItem;

#[async_trait]
pub trait EvidenceRepository: Send + Sync {
    async fn save_evidence(&self, _items: &[EvidenceItem]) -> Result<()> {
        Ok(())
    }
}
