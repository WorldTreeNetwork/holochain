#![allow(missing_docs)]

use std::sync::Arc;

use holochain_types::chain::ChainItem;
use parking_lot::Mutex;
use holo_hash::ActionHash;
use holochain_zome_types::ActionHashed;

use crate::core::{workflow::error::WorkflowResult, validate_chain, SysValidationError};

/// Check sync
pub async fn chc_sync() -> WorkflowResult<()> {
    todo!()
}


pub type TxnId = usize;

trait ChainHeadCoordinator {
    type Item: ChainItem;

    fn head(&self) -> Option<<Self::Item as ChainItem>::Hash>;

    fn add_actions(&self, actions: Vec<Self::Item>) -> Result<(), ChcError>;

    fn get_actions_since_hash(&self, hash: <Self::Item as ChainItem>::Hash) -> Vec<Self::Item>;
}

/// A local Rust implementation of a CHC, for testing purposes only.
#[derive(Clone)]
pub struct LocalChc<A: ChainItem = ActionHashed> {
    actions: Arc<Mutex<Vec<A>>>,
}

impl<A: ChainItem> Default for LocalChc<A> {
    fn default() -> Self {
        Self {
            actions: Arc::new(Mutex::new(Default::default())),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ChcError {
    #[error(transparent)]
    InvalidChain(#[from] SysValidationError),
}

impl<A: ChainItem> LocalChc<A> {
    fn get_head(actions: &Vec<A>) -> Option<A::Hash> {
        actions.last().map(|a| a.item_hash().clone())
    }
}


impl<A: ChainItem> ChainHeadCoordinator for LocalChc<A> {
    type Item = A;

    fn head(&self) -> Option<A::Hash> {
        Self::get_head(&self.actions.lock())
    }

    fn add_actions(&self, new_actions: Vec<A>) -> Result<(), ChcError> {
        let mut actions = self.actions.lock();
        let head = actions.last().map(|a| (a.item_hash().clone(), a.seq()));
        validate_chain(new_actions.iter(), &head)?;
        (*actions).extend(new_actions);
        Ok(())
    }

    fn get_actions_since_hash(&self, hash: A::Hash) -> Vec<A> {
        self.actions.lock().iter().skip_while(|a| a.item_hash() != &hash).cloned().collect()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct TestItem {
        hash: u32,
        prev_hash: Option<u32>,
    }

    impl From<u32> for TestItem {
        fn from(x: u32) -> Self {
            Self {
                hash: x,
                prev_hash: (x > 0).then(|| x - 1),
            }
        }
    }

    impl ChainItem for TestItem {
        type Hash = u32;

        fn prev_hash(&self) -> Option<&u32> {
            self.prev_hash.as_ref()
        }

        fn item_hash(&self) -> &u32 {
            &self.hash
        }

        fn seq(&self) -> u32 {
            // XXX: a little weird, but works if we keep "hashes" sequential
            self.hash
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_add_actions() {
        let chc = LocalChc::default();
        assert_eq!(chc.head(), None);

        fn items(i: impl IntoIterator<Item = u32>) -> Vec<TestItem> {
            i.into_iter().map(TestItem::from).collect()
        } 

        let t0: Vec<TestItem> = items(vec![0, 1, 2]);
        let t1: Vec<TestItem> = items(vec![3, 4, 5]);
        let t2: Vec<TestItem> = items(vec![6, 7, 8]);
        let t99: Vec<TestItem> = items(vec![99]);

        chc.add_actions(t0.clone()).unwrap();
        assert_eq!(chc.head(), Some(2));
        chc.add_actions(t1.clone()).unwrap();
        assert_eq!(chc.head(), Some(5));
        
        // last_hash doesn't match
        assert!(
            chc.add_actions(t0.clone()).is_err()
        );
        assert!(
            chc.add_actions(t1.clone()).is_err()
        );
        assert!(chc.add_actions(t99).is_err());
        assert_eq!(chc.head(), Some(5));
        
        chc.add_actions(t2.clone()).unwrap();
        assert_eq!(chc.head(), Some(8));

        assert_eq!(
            chc.get_actions_since_hash(0),
            items([0,1,2,3,4,5,6,7,8])
        );
        assert_eq!(
            chc.get_actions_since_hash(3),
            items([3,4,5,6,7,8])
        );
        assert_eq!(
            chc.get_actions_since_hash(8),
            items([8])
        );
        assert_eq!(chc.get_actions_since_hash(9), items([]));
    }
}
