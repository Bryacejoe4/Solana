/// NextBlock MEV provider.
///
/// NextBlock uses a similar JSON-RPC bundle API to Jito.
/// Endpoint: https://fra.nextblock.io (or region-specific)
/// Method: `sendBundle` with base58-encoded serialized transactions.

use crate::MevProvider;
use async_trait::async_trait;
use solana_sdk::transaction::Transaction;
use reqwest::Client;
use serde_json::{json, Value};
use tracing::{info, error};
use std::time::Duration;

pub struct NextBlockClient {
    client: Client,
    endpoint_url: String,
    api_key: Option<String>,
}

impl NextBlockClient {
    pub fn new(url: &str, api_key: Option<String>) -> Self {
        Self {
            client: Client::builder().timeout(Duration::from_millis(2000)).build().unwrap(),
            endpoint_url: url.to_string(),
            api_key,
        }
    }

    fn encode_txs(txs: &[Transaction]) -> Vec<String> {
        txs.iter()
            .map(|tx| bincode::serialize(tx).map(|b| bs58::encode(b).into_string()).unwrap_or_default())
            .collect()
    }
}

#[async_trait]
impl MevProvider for NextBlockClient {
    async fn submit_bundle(&self, txs: Vec<Transaction>) -> Result<String, String> {
        let encoded_txs = Self::encode_txs(&txs);

        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendBundle",
            "params": [encoded_txs]
        });

        info!("Sending bundle with {} txs to NextBlock ({})", txs.len(), self.endpoint_url);

        let mut request = self.client.post(&self.endpoint_url).json(&payload);

        // Add API key header if configured
        if let Some(ref key) = self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let resp = request
            .send()
            .await
            .map_err(|e| {
                error!("NextBlock request failed: {}", e);
                format!("NextBlock request failed: {e}")
            })?;

        let txt = resp.text().await.unwrap_or_default();
        info!("NextBlock raw response: {}", txt);

        let parsed: Value = serde_json::from_str(&txt)
            .map_err(|e| format!("NextBlock returned invalid JSON: {e} — raw: {txt}"))?;

        if let Some(err) = parsed.get("error") {
            return Err(format!("NextBlock error: {}", err));
        }

        match parsed.get("result").and_then(|r| r.as_str()) {
            Some(bundle_id) => {
                info!("NextBlock bundle accepted: {}", bundle_id);
                Ok(bundle_id.to_string())
            }
            None => Err(format!("NextBlock response missing 'result': {txt}"))
        }
    }

    fn name(&self) -> &str {
        "NextBlock"
    }
}
