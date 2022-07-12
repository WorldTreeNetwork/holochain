#![allow(missing_docs)]

use std::sync::Arc;

use parking_lot::Mutex;

use holo_hash::ActionHash;
use holochain_zome_types::ActionHashed;

use crate::core::workflow::error::WorkflowResult;

/// Check sync
pub async fn chc_sync() -> WorkflowResult<()> {
    todo!()
}

pub type Transactions<A> = Vec<Vec<A>>;

#[derive(Debug, PartialEq, Eq, derive_more::Constructor)]
pub struct CHCSyncData<A> {
    latest_txn_id: TxnId,
    transactions: Transactions<A>,
}

pub type TxnId = usize;

pub trait ChcItem: Clone + PartialEq + Eq + std::fmt::Debug {
    type Hash: PartialEq + Eq;

    fn prev_hash(&self) -> Option<&Self::Hash>;
    fn as_hash(&self) -> &Self::Hash;
}

impl ChcItem for ActionHashed {
    type Hash = ActionHash;

    fn prev_hash(&self) -> Option<&ActionHash> {
        self.prev_action()
    }

    fn as_hash(&self) -> &ActionHash {
        <ActionHashed as holo_hash::HasHash<_>>::as_hash(self)
    }
}

trait ChainHeadCoordinator {
    type Item: ChcItem;

    fn next_transaction_id(&self) -> TxnId;

    fn add_transaction(&self, txn_id: TxnId, actions: Vec<Self::Item>) -> Result<(), ChcError>;

    fn get_transactions_since_id(&self, txn_id: TxnId) -> Transactions<Self::Item>;
}

/// A local Rust implementation of a CHC, for testing purposes only.
#[derive(Clone)]
pub struct LocalChc<A: ChcItem = ActionHashed> {
    transactions: Arc<Mutex<Transactions<A>>>,
}

impl<A: ChcItem> Default for LocalChc<A> {
    fn default() -> Self {
        Self {
            transactions: Arc::new(Mutex::new(Default::default())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChcError {
    WrongTransactionId,
    HashMismatch,
}

impl<A: ChcItem> ChainHeadCoordinator for LocalChc<A> {
    type Item = A;

    fn next_transaction_id(&self) -> TxnId {
        self.transactions.lock().len().into()
    }

    fn add_transaction(&self, txn_id: TxnId, actions: Vec<A>) -> Result<(), ChcError> {
        let mut txns = self.transactions.lock();
        if txns.len() != txn_id {
            return Err(ChcError::WrongTransactionId);
        }
        let last = txns.last().and_then(|t| t.last());
        if let (Some(last), Some(next)) = (last, actions.first()) {
            if next.prev_hash() != Some(last.as_hash()) {
                return Err(ChcError::HashMismatch);
            }
        }
        (*txns).push(actions);
        Ok(())
    }

    fn get_transactions_since_id(&self, txn_id: TxnId) -> Transactions<A> {
        self.transactions.lock()[txn_id..].to_vec()
    }
}

impl<A: ChcItem> LocalChc<A> {}

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

    impl ChcItem for TestItem {
        type Hash = u32;

        fn prev_hash(&self) -> Option<&u32> {
            self.prev_hash.as_ref()
        }

        fn as_hash(&self) -> &u32 {
            &self.hash
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_add_transaction() {
        let chc = LocalChc::default();
        assert_eq!(chc.next_transaction_id(), 0);

        let t0: Vec<TestItem> = vec![1.into(), 2.into(), 3.into()];
        let t1: Vec<TestItem> = vec![4.into(), 5.into(), 6.into()];
        let t2: Vec<TestItem> = vec![7.into(), 8.into(), 9.into()];
        let t99: Vec<TestItem> = vec![99.into()];

        chc.add_transaction(0, t0.clone()).unwrap();
        assert_eq!(chc.next_transaction_id(), 1);
        chc.add_transaction(1, t1.clone()).unwrap();
        assert_eq!(chc.next_transaction_id(), 2);

        // transaction id isn't correct
        assert_eq!(
            chc.add_transaction(0, t2.clone()),
            Err(ChcError::WrongTransactionId)
        );
        assert_eq!(
            chc.add_transaction(1, t2.clone()),
            Err(ChcError::WrongTransactionId)
        );
        assert_eq!(
            chc.add_transaction(3, t2.clone()),
            Err(ChcError::WrongTransactionId)
        );
        // last_hash doesn't match
        assert_eq!(chc.add_transaction(2, t99), Err(ChcError::HashMismatch));

        chc.add_transaction(2, t2.clone()).unwrap();

        assert_eq!(
            chc.get_transactions_since_id(0),
            vec![t0.clone(), t1.clone(), t2.clone()]
        );
        assert_eq!(
            chc.get_transactions_since_id(1),
            vec![t1.clone(), t2.clone()]
        );
        assert_eq!(chc.get_transactions_since_id(2), vec![t2.clone()]);
    }
}
