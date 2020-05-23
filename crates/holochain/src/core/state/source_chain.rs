//! A SourceChain is guaranteed to be initialized, i.e. it has gone through the CellGenesis workflow.
//! It has the same interface as its underlying SourceChainBuf, except that certain operations,
//! which would return Option in the SourceChainBuf, like getting the source chain head, or the AgentPubKey,
//! cannot fail, so the function return types reflect that.

use holo_hash::*;
use holochain_keystore::Signature;
use holochain_state::{
    buffer::BufferedStore,
    db::GetDb,
    error::DatabaseResult,
    prelude::{Readable, Reader, Writer},
};
use holochain_types::{
    composite_hash::HeaderAddress,
    header::{EntryType, EntryVisibility, HeaderBuilder},
    prelude::*,
    Header, HeaderHashed,
};
use holochain_zome_types::{
    capability::{CapClaim, CapGrant, CapSecret},
    entry::{CapClaimEntry, CapGrantEntry, Entry},
};
use shrinkwraprs::Shrinkwrap;

pub use error::*;
pub use source_chain_buffer::*;

mod error;
mod source_chain_buffer;

/// A wrapper around [SourceChainBuf] with the assumption that the source chain has been initialized,
/// i.e. has undergone Genesis.
#[derive(Shrinkwrap)]
#[shrinkwrap(mutable)]
pub struct SourceChain<'env, R: Readable = Reader<'env>>(pub SourceChainBuf<'env, R>);

impl<'env, R: Readable> SourceChain<'env, R> {
    pub fn agent_pubkey(&self) -> SourceChainResult<AgentPubKey> {
        self.0
            .agent_pubkey()?
            .ok_or(SourceChainError::InvalidStructure(
                ChainInvalidReason::GenesisDataMissing,
            ))
    }

    pub fn chain_head(&self) -> SourceChainResult<&HeaderAddress> {
        self.0.chain_head().ok_or(SourceChainError::ChainEmpty)
    }

    pub fn new(reader: &'env R, dbs: &impl GetDb) -> DatabaseResult<Self> {
        Ok(SourceChainBuf::new(reader, dbs)?.into())
    }

    pub fn into_inner(self) -> SourceChainBuf<'env, R> {
        self.0
    }

    pub async fn put_cap_grant(
        &mut self,
        grant_entry: CapGrantEntry,
    ) -> SourceChainResult<HeaderAddress> {
        let entry = Entry::CapGrant(grant_entry);
        let entry_hash = EntryContentHash::with_data(SerializedBytes::try_from(&entry)?.bytes())
            .await
            .into();
        let header_builder = HeaderBuilder::EntryCreate {
            entry_type: EntryType::CapGrant,
            entry_hash,
        };
        self.put(header_builder, Some(entry)).await
    }

    pub async fn put_cap_claim(
        &mut self,
        claim_entry: CapClaimEntry,
    ) -> SourceChainResult<HeaderAddress> {
        let entry = Entry::CapClaim(claim_entry);
        let entry_hash = EntryContentHash::with_data(SerializedBytes::try_from(&entry)?.bytes())
            .await
            .into();
        let header_builder = HeaderBuilder::EntryCreate {
            entry_type: EntryType::CapClaim,
            entry_hash,
        };
        self.put(header_builder, Some(entry)).await
    }

    pub fn get_persisted_cap_grant_by_secret(
        &self,
        query: &CapSecret,
    ) -> SourceChainResult<Option<CapGrant>> {
        let hashes_n_grants: Vec<_> = self
            .0
            .cas()
            .private_entries()
            .expect("SourceChainBuf must have access to private entries")
            .iter_raw()?
            .filter_map(|(key, entry)| {
                entry.as_cap_grant().and_then(|grant| {
                    grant.access().secret().and_then(|secret| {
                        if secret == query {
                            let hash = tokio_safe_block_on::tokio_safe_block_on(
                                async { EntryContentHash::with_pre_hashed(key.to_owned()).await },
                                std::time::Duration::from_millis(10),
                            );
                            Some((hash, grant))
                        } else {
                            None
                        }
                    })
                })
            })
            .collect();

        let answer = if hashes_n_grants.len() == 0 {
            None
        } else if hashes_n_grants.len() == 1 {
            hashes_n_grants.first().map(|p| p.1.clone())
        } else {
            // TODO: we SHOULD iterate through the chain now to find the most
            // recent grant with this secret, in case it was updated.
            // This will be handled in the future with an index, for simple
            // lookup by secret
            todo!("Find proper grant or implement capability index")
        };
        Ok(answer)
    }

    pub fn get_persisted_cap_claim_by_secret(
        &self,
        query: &CapSecret,
    ) -> SourceChainResult<Option<CapClaim>> {
        let hashes_n_claims: Vec<_> = self
            .0
            .cas()
            .private_entries()
            .expect("SourceChainBuf must have access to private entries")
            .iter_raw()?
            .filter_map(|(key, entry)| {
                entry.as_cap_claim().and_then(|claim| {
                    if claim.secret() == query {
                        let hash = tokio_safe_block_on::tokio_safe_block_on(
                            async { EntryContentHash::with_pre_hashed(key.to_owned()).await },
                            std::time::Duration::from_millis(10),
                        );
                        Some((hash, claim.clone()))
                    } else {
                        None
                    }
                })
            })
            .collect();

        let answer = if hashes_n_claims.len() == 0 {
            None
        } else if hashes_n_claims.len() == 1 {
            hashes_n_claims.first().map(|p| p.1.clone())
        } else {
            // TODO: we SHOULD iterate through the chain now to find the most
            // recent claim with this secret, in case it was updated.
            // This will be handled in the future with an index, for simple
            // lookup by secret
            todo!("Find proper claim or implement capability index")
        };
        Ok(answer)
    }
}

impl<'env, R: Readable> From<SourceChainBuf<'env, R>> for SourceChain<'env, R> {
    fn from(buffer: SourceChainBuf<'env, R>) -> Self {
        Self(buffer)
    }
}

impl<'env, R: Readable> BufferedStore<'env> for SourceChain<'env, R> {
    type Error = SourceChainError;

    fn flush_to_txn(self, writer: &'env mut Writer) -> Result<(), Self::Error> {
        self.0.flush_to_txn(writer)?;
        Ok(())
    }
}

/// a chain element which is a triple containing the signature of the header along with the
/// entry if the header type has one.
#[derive(Clone, Debug, PartialEq)]
pub struct ChainElement {
    signed_header: SignedHeaderHashed,
    maybe_entry: Option<Entry>,
}

impl ChainElement {
    /// Raw element constructor.  Used only when we know that the values are valid.
    pub fn new(signed_header: SignedHeaderHashed, maybe_entry: Option<Entry>) -> Self {
        Self {
            signed_header,
            maybe_entry,
        }
    }

    pub fn into_inner(self) -> (SignedHeaderHashed, Option<Entry>) {
        (self.signed_header, self.maybe_entry)
    }

    /// Validates a chain element
    pub async fn validate(&self) -> SourceChainResult<()> {
        self.signed_header.validate().await?;

        //TODO: make sure that any cases around entry existence are valid:
        //      SourceChainError::InvalidStructure(HeaderAndEntryMismatch(address)),
        Ok(())
    }

    /// Access the signature portion of this triple.
    pub fn signature(&self) -> &Signature {
        self.signed_header.signature()
    }

    /// Access the header address
    pub fn header_address(&self) -> &HeaderAddress {
        self.signed_header.header_address()
    }

    /// Access the Header portion of this triple.
    pub fn header(&self) -> &Header {
        self.signed_header.header()
    }

    /// Access the HeaderHashed portion.
    pub fn header_hashed(&self) -> &HeaderHashed {
        self.signed_header.header_hashed()
    }

    /// Access the Entry portion of this triple as a ChainElementEntry,
    /// which includes the context around the presence or absence of the entry.
    pub fn entry(&self) -> ChainElementEntry {
        let maybe_visibilty = self
            .header()
            .entry_data()
            .map(|(_, entry_type)| entry_type.visibility());
        match (self.maybe_entry.as_ref(), maybe_visibilty) {
            (Some(entry), Some(_)) => ChainElementEntry::Present(entry),
            (None, Some(EntryVisibility::Private)) => ChainElementEntry::Hidden,
            (None, None) => ChainElementEntry::NotApplicable,
            (Some(_), None) => {
                unreachable!("Entry is present for a Header type which has no entry reference")
            }
            (None, Some(EntryVisibility::Public)) => unreachable!("Entry data missing for element"),
        }
    }
}

/// Represents the different ways the entry_address reference within a Header
/// can be intepreted
#[derive(Clone, Debug, PartialEq, Eq, derive_more::From)]
pub enum ChainElementEntry<'a> {
    /// The Header has an entry_address reference, and the Entry is accessible.
    Present(&'a Entry),
    /// The Header has an entry_address reference, but we are in a public
    /// context and the entry is private.
    Hidden,
    /// The Header does not contain an entry_address reference.
    NotApplicable,
}

impl<'a> ChainElementEntry<'a> {
    pub fn as_option(&'a self) -> Option<&'a Entry> {
        if let ChainElementEntry::Present(entry) = self {
            Some(entry)
        } else {
            None
        }
    }
}

/// the header and the signature that signed it
#[derive(Clone, Debug, PartialEq)]
pub struct SignedHeaderHashed {
    header: HeaderHashed,
    signature: Signature,
}

impl SignedHeaderHashed {
    /// SignedHeader constructor
    pub async fn new(keystore: &KeystoreSender, header: HeaderHashed) -> SourceChainResult<Self> {
        let signature = header.author().sign(keystore, &*header).await?;
        Ok(Self::with_presigned(header, signature))
    }

    /// Constructor for an already signed header
    pub fn with_presigned(header: HeaderHashed, signature: Signature) -> Self {
        Self { header, signature }
    }

    pub fn into_inner(self) -> (HeaderHashed, Signature) {
        (self.header, self.signature)
    }

    /// Access the Header Hash.
    pub fn header_address(&self) -> &HeaderAddress {
        self.header.as_hash()
    }

    /// Access the Header portion.
    pub fn header(&self) -> &Header {
        &*self.header
    }

    /// Access the HeaderHashed portion.
    pub fn header_hashed(&self) -> &HeaderHashed {
        &self.header
    }

    /// Access the signature portion.
    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    /// Validates a signed header
    pub async fn validate(&self) -> SourceChainResult<()> {
        if !self
            .header
            .author()
            .verify_signature(&self.signature, &*self.header)
            .await?
        {
            return Err(SourceChainError::InvalidSignature);
        }
        Ok(())
    }
}

#[cfg(test)]
pub mod tests {

    use super::*;
    use holochain_state::prelude::*;
    use holochain_state::test_utils::test_cell_env;
    use holochain_types::test_utils::{fake_agent_pubkey_1, fake_dna_hash};
    use holochain_zome_types::capability::{CapAccess, ZomeCallCapGrant};
    use std::collections::HashMap;

    #[tokio::test(threaded_scheduler)]
    async fn test_get_cap_grant() -> SourceChainResult<()> {
        let arc = test_cell_env();
        let env = arc.guard().await;
        let access = CapAccess::transferable();
        let secret = access.secret().unwrap();
        let grant = ZomeCallCapGrant::new("tag".into(), access.clone(), HashMap::new());
        {
            let reader = env.reader()?;
            let mut store = SourceChainBuf::new(&reader, &env)?;
            store
                .genesis(fake_dna_hash(""), fake_agent_pubkey_1(), None)
                .await?;
            env.with_commit(|writer| store.flush_to_txn(writer))?;
        }

        {
            let reader = env.reader()?;
            let mut chain = SourceChain::new(&reader, &env)?;
            chain.put_cap_grant(grant.clone()).await?;

            // ideally the following would work, but it won't because currently
            // we can't get grants from the scratch space
            // this will be fixed once we add the capability index

            // assert_eq!(
            //     chain.get_persisted_cap_grant_by_secret(secret)?,
            //     Some(grant.clone().into())
            // );

            env.with_commit(|writer| chain.flush_to_txn(writer))?;
        }

        {
            let reader = env.reader()?;
            let chain = SourceChain::new(&reader, &env)?;
            assert_eq!(
                chain.get_persisted_cap_grant_by_secret(secret)?,
                Some(grant.into())
            );
        }

        Ok(())
    }

    #[tokio::test(threaded_scheduler)]
    async fn test_get_cap_claim() -> SourceChainResult<()> {
        let arc = test_cell_env();
        let env = arc.guard().await;
        let secret = CapSecret::random();
        let agent_pubkey = fake_agent_pubkey_1().into();
        let claim = CapClaim::new("tag".into(), agent_pubkey, secret.clone());
        {
            let reader = env.reader()?;
            let mut store = SourceChainBuf::new(&reader, &env)?;
            store
                .genesis(fake_dna_hash(""), fake_agent_pubkey_1(), None)
                .await?;
            env.with_commit(|writer| store.flush_to_txn(writer))?;
        }

        {
            let reader = env.reader()?;
            let mut chain = SourceChain::new(&reader, &env)?;
            chain.put_cap_claim(claim.clone()).await?;

            // ideally the following would work, but it won't because currently
            // we can't get claims from the scratch space
            // this will be fixed once we add the capability index

            // assert_eq!(
            //     chain.get_persisted_cap_claim_by_secret(&secret)?,
            //     Some(claim.clone())
            // );

            env.with_commit(|writer| chain.flush_to_txn(writer))?;
        }

        {
            let reader = env.reader()?;
            let chain = SourceChain::new(&reader, &env)?;
            assert_eq!(
                chain.get_persisted_cap_claim_by_secret(&secret)?,
                Some(claim)
            );
        }

        Ok(())
    }
}
