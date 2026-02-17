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

    pub async fn execute(&self, plan: TradePlan) -> EngineResult<String> {
        let kp_path = shellexpand::tilde(&self.keypair_path).to_string();
        let payer = read_keypair_file(&kp_path)
            .map_err(|e| EngineError::Config(format!("keypair load failed: {e}")))?;

        let bh = self.rpc.get_latest_blockhash()
            .map_err(|e| EngineError::Rpc(format!("blockhash: {e}")))?;

        let mut tx = Transaction::new_with_payer(&plan.instructions, Some(&payer.pubkey()));
        tx.sign(&[&payer], bh);

        if plan.simulate {
            let sim = self.rpc.simulate_transaction(&tx)
                .map_err(|e| EngineError::Rpc(format!("simulate rpc: {e}")))?;
            if let Some(err) = sim.value.err {
                // Return verification error here? Or just log?
                // For high perf, we might want to continue or bail.
                // Returning error per spec:
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

        let sig = self.rpc.send_transaction(&tx)
            .map_err(|e| EngineError::Rpc(format!("send tx: {e}")))?;

        info!(%sig, "Submitted transaction via RPC");
        Ok(sig.to_string())
    }
}
