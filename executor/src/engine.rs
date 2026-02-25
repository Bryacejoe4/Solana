use common::error::{EngineError, EngineResult};
use common::types::TradePlan;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    signature::{read_keypair_file, Signer},
    transaction::Transaction,
};
use tracing::{info, warn};
use mev::jito::JitoClient;

pub struct ExecutionEngine {
    rpc: RpcClient,
    keypair_path: String,
    jito_client: Option<JitoClient>,
}

impl ExecutionEngine {
    pub fn new(rpc_http_url: String, keypair_path: String, jito_url: Option<String>) -> Self {
        let jito_client = jito_url.map(|url| JitoClient::new(&url));
        Self { 
            rpc: RpcClient::new(rpc_http_url), 
            keypair_path,
            jito_client, 
        }
    }

    pub fn rpc_client(&self) -> &RpcClient {
        &self.rpc
    }

    pub async fn execute(&self, plan: TradePlan) -> EngineResult<String> {
        let kp_path = shellexpand::tilde(&self.keypair_path).to_string();
        let payer = read_keypair_file(&kp_path)
            .unwrap_or_else(|_| solana_sdk::signature::Keypair::from_base58_string(&self.keypair_path));

        let bh = self.rpc.get_latest_blockhash()
            .map_err(|e| EngineError::Rpc(format!("blockhash: {e}")))?;

        let mut tx = Transaction::new_with_payer(&plan.instructions, Some(&payer.pubkey()));
        tx.sign(&[&payer], bh);

        if plan.simulate {
            let sim = self.rpc.simulate_transaction(&tx)
                .map_err(|e| EngineError::Rpc(format!("simulate rpc: {e}")))?;
            if let Some(err) = sim.value.err {
                if let Some(logs) = sim.value.logs {
                    for log in logs {
                        warn!("SIMULATION LOG: {}", log);
                    }
                }
                return Err(EngineError::Simulation(format!("{err:?}")));
            }
            info!("Simulation successful");
        }

        if let Some(jito) = &self.jito_client {
            info!("Submitting via Jito Block Engine (High Performance)...");
            match jito.submit_bundle(vec![tx.clone()]).await {
                Ok(bundle_id) => {
                    info!("Bundle submitted. ID: {}", bundle_id);
                    return Ok(bundle_id);
                }
                Err(e) => {
                    warn!("Jito submission failed: {}. Falling back to RPC.", e);
                }
            }
        }

        let config = solana_client::rpc_config::RpcSendTransactionConfig {
            skip_preflight: true,
            ..Default::default()
        };
        let sig = self.rpc.send_transaction_with_config(&tx, config)
            .map_err(|e| EngineError::Rpc(format!("send tx: {e}")))?;

        info!(%sig, "Submitted transaction via RPC");
        Ok(sig.to_string())
    }
}
