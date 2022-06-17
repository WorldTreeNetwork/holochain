//! An Entry is a unit of data in a Holochain Source Chain.
//!
//! This module contains all the necessary definitions for Entry, which broadly speaking
//! refers to any data which will be written into the ContentAddressableStorage, or the EntityAttributeValueStorage.
//! It defines serialization behaviour for entries. Here you can find the complete list of
//! entry_types, and special entries, like deletion_entry and cap_entry.

use crate::action::ChainTopOrdering;
use holochain_integrity_types::EntryDefIndex;
use holochain_serialized_bytes::prelude::*;

mod app_entry_bytes;
pub use app_entry_bytes::*;

pub use holochain_integrity_types::entry::*;
use holochain_wasmer_common::WasmError;

#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
/// Either an [`EntryDefIndex`] or one of:
/// - [`EntryType::CapGrant`](crate::prelude::EntryType::CapGrant)
/// - [`EntryType::CapClaim`](crate::prelude::EntryType::CapClaim)
/// - [`EntryType::AgentPubKey`](crate::prelude::EntryType::AgentPubKey)
/// Which don't have an index.
pub enum EntryDefLocation {
    /// [`crate::EntryType::AgentPubKey`] is committed to and
    /// validated by all integrity zomes in the dna.
    Agent,
    /// App defined entries always have a unique [`u8`] index
    /// within the Dna.
    App(EntryDefIndex),
    /// [`crate::EntryType::CapClaim`] is committed to and
    /// validated by all integrity zomes in the dna.
    CapClaim,
    /// [`crate::EntryType::CapGrant`] is committed to and
    /// validated by all integrity zomes in the dna.
    CapGrant,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
/// Options for controlling how get works
pub struct GetOptions {
    /// If this is true the get call will wait for
    /// the latest data before returning.
    /// If it is false you will get whatever is locally
    /// available on this conductor.
    pub strategy: GetStrategy,
}

impl GetOptions {
    /// This will get you the content
    /// with latest metadata if it can
    /// otherwise it will fallback to what
    /// you have cached locally.
    ///
    /// This call is guaranteed to not go to
    /// the network if you are an authority
    /// for this hash.
    pub fn latest() -> Self {
        Self {
            strategy: GetStrategy::Latest,
        }
    }
    /// Gets the content but does not
    /// try to get the latest metadata.
    /// This will save a network call if the
    /// entry is local (cached, authored or integrated).
    ///
    /// This will fallback to the network if the content
    /// is not found locally
    pub fn content() -> Self {
        Self {
            strategy: GetStrategy::Content,
        }
    }
}

impl Default for GetOptions {
    fn default() -> Self {
        Self::latest()
    }
}

#[derive(PartialEq, Debug, Clone, Copy, Serialize, Deserialize)]
/// Describes the get call and what information
/// the caller is concerned about.
/// This helps the subconscious avoid unnecessary network calls.
pub enum GetStrategy {
    /// Will try to get the latest metadata but fallback
    /// to the cache if none is found.
    /// Does not go to the network if you are an authority for the data.
    Latest,
    /// Will try to get the content locally but go
    /// to the network if it is not found.
    /// Does not go to the network if you are an authority for the data.
    Content,
}

/// Zome input to create an entry.
#[derive(PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize, SerializedBytes)]
pub struct CreateInput {
    /// The data for creating this element.
    pub builder: RecordBuilder,
    /// ChainTopBehaviour for the write.
    pub chain_top_ordering: ChainTopOrdering,
}

impl CreateInput {
    /// Constructor.
    pub fn new<E>(
        builder: impl TryInto<RecordBuilder, Error = E>,
        chain_top_ordering: ChainTopOrdering,
    ) -> Result<Self, WasmError>
    where
        WasmError: From<E>,
    {
        Ok(Self {
            builder: builder.try_into()?,
            chain_top_ordering,
        })
    }

    /// Consume into an Entry.
    pub fn into_entry(self) -> Entry {
        self.builder.into()
    }

    /// Accessor.
    pub fn chain_top_ordering(&self) -> &ChainTopOrdering {
        &self.chain_top_ordering
    }
}

/// Zome input for get and get_details calls.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct GetInput {
    /// Any DHT hash to pass to get or get_details.
    pub any_dht_hash: holo_hash::AnyDhtHash,
    /// Options for the call.
    pub get_options: crate::entry::GetOptions,
}

impl GetInput {
    /// Constructor.
    pub fn new(any_dht_hash: holo_hash::AnyDhtHash, get_options: crate::entry::GetOptions) -> Self {
        Self {
            any_dht_hash,
            get_options,
        }
    }
}

/// Zome input type for all update operations.
#[derive(PartialEq, Debug, Deserialize, Serialize, Clone)]
pub struct UpdateInput {
    /// Action of the record being updated.
    pub original_action_address: holo_hash::ActionHash,
    /// Entry body.
    pub entry: crate::entry::Entry,
    /// ChainTopBehaviour for the write.
    pub chain_top_ordering: ChainTopOrdering,
}

/// Zome input for all delete operations.
#[derive(PartialEq, Debug, Deserialize, Serialize, Clone)]
pub struct DeleteInput {
    /// Action of the record being deleted.
    pub deletes_action_hash: holo_hash::ActionHash,
    /// Chain top ordering behaviour for the delete.
    pub chain_top_ordering: ChainTopOrdering,
}

impl DeleteInput {
    /// Constructor.
    pub fn new(
        deletes_action_hash: holo_hash::ActionHash,
        chain_top_ordering: ChainTopOrdering,
    ) -> Self {
        Self {
            deletes_action_hash,
            chain_top_ordering,
        }
    }
}

impl From<holo_hash::ActionHash> for DeleteInput {
    /// Sets [`ChainTopOrdering`] to `default` = `Strict` when created from a hash.
    fn from(deletes_action_hash: holo_hash::ActionHash) -> Self {
        Self {
            deletes_action_hash,
            chain_top_ordering: ChainTopOrdering::default(),
        }
    }
}

impl EntryDefLocation {
    /// Create an [`EntryDefLocation::App`].
    pub fn app(entry_def_index: impl Into<EntryDefIndex>) -> Self {
        Self::App(entry_def_index.into())
    }
}

impl From<EntryDefIndex> for EntryDefLocation {
    fn from(i: EntryDefIndex) -> Self {
        EntryDefLocation::App(i)
    }
}
