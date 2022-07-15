//! Types for Chain Head Coordination

mod chc_remote;

use holochain_types::prelude::*;

use crate::core::{validate_chain};

/// A local Rust implementation of a CHC, for testing purposes only.
pub struct LocalChc<A: ChainItem = SignedActionHashed> {
    actions: Vec<A>,
}

impl<A: ChainItem> Default for LocalChc<A> {
    fn default() -> Self {
        Self {
            actions: Default::default(),
        }
    }
}

impl<A: ChainItem> LocalChc<A> {
    fn get_head(actions: &Vec<A>) -> Option<A::Hash> {
        actions.last().map(|a| a.item_hash().clone())
    }
}

#[async_trait::async_trait]
impl<A: ChainItem> ChainHeadCoordinator for LocalChc<A> {
    type Item = A;

    async fn head(&self) -> ChcResult<Option<A::Hash>> {
        Ok(Self::get_head(&self.actions))
    }

    async fn add_actions(&mut self, new_actions: Vec<A>) -> ChcResult<()> {
        let head = self.actions.last().map(|a| (a.item_hash().clone(), a.seq()));
        validate_chain(new_actions.iter(), &head).map_err(|e| ChcError::InvalidChain(e.to_string()))?;
        self.actions.extend(new_actions);
        Ok(())
    }

    async fn get_actions_since_hash(&self, hash: A::Hash) -> ChcResult<Vec<A>> {
        Ok(self.actions.iter().skip_while(|a| a.item_hash() != &hash).cloned().collect())
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
        let mut chc = LocalChc::default();
        assert_eq!(chc.head().await.unwrap(), None);

        fn items(i: impl IntoIterator<Item = u32>) -> Vec<TestItem> {
            i.into_iter().map(TestItem::from).collect()
        } 

        let t0: Vec<TestItem> = items(vec![0, 1, 2]);
        let t1: Vec<TestItem> = items(vec![3, 4, 5]);
        let t2: Vec<TestItem> = items(vec![6, 7, 8]);
        let t99: Vec<TestItem> = items(vec![99]);

        chc.add_actions(t0.clone()).await.unwrap();
        assert_eq!(chc.head().await.unwrap(), Some(2));
        chc.add_actions(t1.clone()).await.unwrap();
        assert_eq!(chc.head().await.unwrap(), Some(5));
        
        // last_hash doesn't match
        assert!(
            chc.add_actions(t0.clone()).await.is_err()
        );
        assert!(
            chc.add_actions(t1.clone()).await.is_err()
        );
        assert!(chc.add_actions(t99).await.is_err());
        assert_eq!(chc.head().await.unwrap(), Some(5));
        
        chc.add_actions(t2.clone()).await.unwrap();
        assert_eq!(chc.head().await.unwrap(), Some(8));

        assert_eq!(
            chc.get_actions_since_hash(0).await.unwrap(),
            items([0,1,2,3,4,5,6,7,8])
        );
        assert_eq!(
            chc.get_actions_since_hash(3).await.unwrap(),
            items([3,4,5,6,7,8])
        );
        assert_eq!(
            chc.get_actions_since_hash(8).await.unwrap(),
            items([8])
        );
        assert_eq!(chc.get_actions_since_hash(9).await.unwrap(), items([]));
    }
}
