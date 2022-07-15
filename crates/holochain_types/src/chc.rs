#![allow(missing_docs)]

use holochain_serialized_bytes::SerializedBytesError;

use crate::chain::ChainItem;

#[async_trait::async_trait]
pub trait ChainHeadCoordinator {
    type Item: ChainItem;

    async fn head(&self) -> ChcResult<Option<<Self::Item as ChainItem>::Hash>>;

    async fn add_actions(&mut self, actions: Vec<Self::Item>) -> ChcResult<()>;

    async fn get_actions_since_hash(&self, hash: <Self::Item as ChainItem>::Hash) -> ChcResult<Vec<Self::Item>>;
}

#[derive(Debug, thiserror::Error)]
pub enum ChcError {
    #[error("The CHC service is unreachable: {0}")]
    ServiceUnreachable(#[from] reqwest::Error),

    #[error("Adding these actions to the CHC results in an invalid chain. Error: {0}")]
    InvalidChain(String),

    #[error(transparent)]
    DeserializationError(#[from] SerializedBytesError)
}

pub type ChcResult<T> = Result<T, ChcError>;