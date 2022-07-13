//! Types related to an agents for chain activity
use crate::activity::AgentActivityResponse;
use crate::activity::ChainItems;
use holo_hash::ActionHash;
use holo_hash::AgentPubKey;
use holochain_zome_types::ActionHashed;
use holochain_zome_types::prelude::ChainStatus;

/// Helpers for constructing AgentActivity
pub trait AgentActivityExt {
    /// Create an empty chain status
    fn empty<T>(agent: &AgentPubKey) -> AgentActivityResponse<T> {
        AgentActivityResponse {
            agent: agent.clone(),
            valid_activity: ChainItems::NotRequested,
            rejected_activity: ChainItems::NotRequested,
            status: ChainStatus::Empty,
            // TODO: Add the actual highest observed in a follow up PR
            highest_observed: None,
        }
    }
}

impl AgentActivityExt for AgentActivityResponse {}

/// Abstraction of a source chain item, exposing only the parts that the chain cares about.
/// The main implementation of this is `ActionHashed`
pub trait ChainItem: Clone + PartialEq + Eq + std::fmt::Debug {
    /// The hash associated with this item
    type Hash: Clone + PartialEq + Eq;

    /// Get the previous hash in the chain
    fn prev_hash(&self) -> Option<&Self::Hash>;
    /// Get the hash of this item
    fn item_hash(&self) -> &Self::Hash;
    /// The sequence in the chain of this item
    fn seq(&self) -> u32;

}

impl ChainItem for ActionHashed {
    type Hash = ActionHash;

    fn prev_hash(&self) -> Option<&ActionHash> {
        self.prev_action()
    }

    fn item_hash(&self) -> &ActionHash {
        <ActionHashed as holo_hash::HasHash<_>>::as_hash(self)
    }

    fn seq(&self) -> u32 {
        self.action_seq()
    }
}
