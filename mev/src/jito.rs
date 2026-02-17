use solana_sdk::transaction::Transaction;
use reqwest::Client;
use serde_json::json;
use tracing::{info, error};
use std::time::Duration;

pub struct JitoClient {
    client: Client,
    block_engine_url: String,
}

impl JitoClient {
    pub fn new(url: &str) -> Self {
        Self {
            client: Client::builder().timeout(Duration::from_millis(500)).build().unwrap(),
            block_engine_url: url.to_string(),
        }
    }

    pub async fn submit_bundle(&self, txs: Vec<Transaction>) -> Result<String, String> {
        // Real Jito JSON-RPC (sendBundle)
        // In a real impl, we serialize txs to base58 or base64
        // This removes the "Stub" and puts in the actual network call structure.
        
        let encoded_txs: Vec<String> = txs.iter()
            .map(|tx| bincode::serialize(tx).map(|b| bs58::encode(b).into_string()).unwrap_or_default())
            .collect();

        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendBundle",
            "params": [
                encoded_txs
            ]
        });

        info!("Sending bundle with {} txs to {}", txs.len(), self.block_engine_url);

        let resp_result = self.client.post(&self.block_engine_url)
            .json(&payload)
            .send()
            .await;

        match resp_result {
            Ok(resp) => {
                let txt = resp.text().await.unwrap_or_default();
                info!("Jito Response: {}", txt);
                Ok(txt)
            }
            Err(e) => {
                error!("Jito Check Failed: {}", e);
                Err(e.to_string())
            }
        }
    }
}
