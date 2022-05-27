//! Rate limiting data types

use holochain_serialized_bytes::prelude::*;

use crate::{CreateLink, Entry};

mod bucket;
pub use bucket::*;

/// Input to the `weigh` callback
#[derive(Clone, PartialEq, Serialize, Deserialize, SerializedBytes, Debug)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum WeighInput {
    /// A Link to be weighed
    Link(CreateLink),
    /// An Entry to be weighed (TODO: include header as well?)
    Entry(Entry),
}

/// A bucket ID, for rate limiting
pub type RateBucketId = u8;

/// The weight of this header, for rate limiting
pub type RateWeight = u8;

/// The normalized total size of this header, for rate limiting
pub type RateBytes = u8;

/// Combination of two rate limiting data types, for convenience
#[derive(
    Debug,
    Clone,
    serde::Serialize,
    serde::Deserialize,
    PartialEq,
    Eq,
    SerializedBytes,
    Hash,
    PartialOrd,
    Ord,
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[allow(missing_docs)]
pub struct LinkWeight {
    pub rate_bucket: RateBucketId,
    pub rate_weight: RateWeight,
}

impl Default for LinkWeight {
    fn default() -> Self {
        Self {
            rate_bucket: 255,
            rate_weight: 0,
        }
    }
}

/// Combination of the three main rate limiting data types, for convenience
#[derive(
    Debug,
    Clone,
    serde::Serialize,
    serde::Deserialize,
    PartialEq,
    Eq,
    SerializedBytes,
    Hash,
    PartialOrd,
    Ord,
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[allow(missing_docs)]
pub struct EntryWeight {
    pub rate_bucket: RateBucketId,
    pub rate_weight: RateWeight,
    pub rate_bytes: RateBytes,
}

impl Default for EntryWeight {
    fn default() -> Self {
        Self {
            rate_bucket: 255,
            rate_weight: 0,
            rate_bytes: 0,
        }
    }
}
