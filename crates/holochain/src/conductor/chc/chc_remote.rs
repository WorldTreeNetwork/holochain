use holo_hash::ActionHash;
use holochain_p2p::ChcImpl;
use holochain_types::chc::{ChainHeadCoordinator, ChcResult};
use holochain_zome_types::prelude::*;
use reqwest::Url;
use ::bytes::Bytes;
use holochain_serialized_bytes::{encode, decode};

/// An HTTP client which can talk to a remote CHC implementation
pub struct ChcRemote {
    base_url: url::Url
}

#[async_trait::async_trait]
impl ChainHeadCoordinator for ChcRemote {
    type Item = SignedActionHashed;

    async fn head(&self) -> ChcResult<Option<ActionHash>> {
        let response = self.get("/head").await?;
        Ok(decode(&response)?)
    }

    async fn add_actions(&mut self, actions: Vec<Self::Item>) -> ChcResult<()> {
        let body = encode(&actions)?;
        let response = self.post("/add_actions", body).await?;
        Ok(())
    }

    async fn get_actions_since_hash(&self, hash: ActionHash) -> ChcResult<Vec<Self::Item>> {
        let body = encode(&hash)?;
        let response = self.post("/get_actions_since_hash", body).await?;
        Ok(decode(&response)?)
    }
}

impl ChcRemote {

    /// Constructor
    pub fn new(namespace: &str, cell_id: &CellId) -> Self {
        todo!()
    }

    fn url(&self, path: &str) -> Url {
        assert!(path.chars().nth(0) == Some('/'));
        Url::parse(&format!("{}{}", self.base_url, path)).expect("invalid URL")
    }

    async fn get(&self, path: &str) -> ChcResult<Bytes> {
        let bytes = reqwest::get(self.url(path))
            .await?
            .bytes()
            .await?;
        Ok(bytes)
    }

    async fn post(&self, path: &str, body: Vec<u8>) -> ChcResult<Bytes> {
        let client = reqwest::Client::new();
        let response = client.post(self.url("/add_actions")).body(body).send().await?;
        Ok(response.bytes().await?)
    }
}