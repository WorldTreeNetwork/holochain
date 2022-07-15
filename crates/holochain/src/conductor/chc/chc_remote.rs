use holo_hash::ActionHash;
use holochain_types::chc::{ChainHeadCoordinator, ChcResult};
use holochain_zome_types::prelude::*;
use reqwest::Url;
use ::bytes::Bytes;
use holochain_serialized_bytes::{encode, decode};

pub struct ChcRemote {
    base_url: url::Url
}

#[async_trait::async_trait]
impl ChainHeadCoordinator for ChcRemote {
    type Item = SignedActionHashed;

    async fn head(&self) -> ChcResult<Option<ActionHash>> {
        Ok(decode(&self.get("/head").await?)?)
    }

    async fn add_actions(&mut self, actions: Vec<Self::Item>) -> ChcResult<()> {
        let client = reqwest::Client::new();
        let body = encode(&actions)?;
        
        Ok(())
    }

    async fn get_actions_since_hash(&self, hash: ActionHash) -> ChcResult<Vec<Self::Item>> {
        Ok(decode(&self.post("/get_actions_since_hash", encode(&hash)?).await?)?)
    }
}

impl ChcRemote {
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