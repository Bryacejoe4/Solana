
use common::error::{EngineError, EngineResult};
use common::types::TradePlan;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    signature::{read_keypair_file, Signer},
    transaction::Transaction,
};
use tracing::info;

pub struct ExecutionEngine {
    rpc: RpcClient,
    keypair_path: String,
}

impl ExecutionEngine {
    pub fn new(rpc_http_url: String, keypair_path: String) -> Self {
        Self { rpc: RpcClient::new(rpc_http_url), keypair_path }
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
                return Err(EngineError::Simulation(format!("{err:?}")));
            }
        }

        let sig = self.rpc.send_transaction(&tx)
            .map_err(|e| EngineError::Rpc(format!("send tx: {e}")))?;

        info!(%sig, "submitted transaction");
        Ok(sig.to_string())
    }
}
