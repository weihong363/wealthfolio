use async_trait::async_trait;

use crate::errors::Result;

use super::models::EvidenceItem;

#[async_trait]
pub trait EvidenceProvider: Send + Sync {
    async fn fetch_evidence(&self) -> Result<Vec<EvidenceItem>>;
}

pub struct EmptyEvidenceProvider;

#[async_trait]
impl EvidenceProvider for EmptyEvidenceProvider {
    async fn fetch_evidence(&self) -> Result<Vec<EvidenceItem>> {
        Ok(Vec::new())
    }
}
