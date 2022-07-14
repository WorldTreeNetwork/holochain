#![allow(missing_docs)]

use crate::chain::ChainItem;
pub trait ChainHeadCoordinator {
    type Item: ChainItem;

    fn head(&self) -> Option<<Self::Item as ChainItem>::Hash>;

    fn add_actions(&mut self, actions: Vec<Self::Item>) -> Result<(), ChcError>;

    fn get_actions_since_hash(&self, hash: <Self::Item as ChainItem>::Hash) -> Vec<Self::Item>;
}

#[derive(Debug, thiserror::Error)]
pub enum ChcError {
    #[error("Adding these actions to the CHC results in an invalid chain. Error: {0}")]
    InvalidChain(String),
}
