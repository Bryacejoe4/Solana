use common::error::{EngineError, EngineResult};
use common::types::TradePlan;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    signature::{read_keypair_file, Signature, Signer, Keypair},
    transaction::Transaction,
    commitment_config::CommitmentConfig,
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use mev::MevProvider;

// ── Configuration Constants ──────────────────────────────────────────────────

/// Maximum time to wait for on-chain confirmation before giving up.
const CONFIRM_TIMEOUT: Duration = Duration::from_secs(30);
/// How often to poll for confirmation status.
const CONFIRM_POLL_INTERVAL: Duration = Duration::from_millis(500);
/// Maximum number of retry attempts for transient failures.
const MAX_RETRIES: u32 = 3;
/// Base delay between retries (exponential backoff).
const RETRY_BASE_DELAY: Duration = Duration::from_millis(200);

// ── Execution Engine ─────────────────────────────────────────────────────────

pub struct ExecutionEngine {
    rpc: Arc<RpcClient>,
    keypair_path: String,
    /// MEV bundle submission provider (could be a single provider or a router).
    mev_provider: Option<Arc<dyn MevProvider>>,
    /// Active cancellation tokens for in-flight trades, keyed by trade identifier.
    active_trades: Arc<Mutex<std::collections::HashMap<String, CancellationToken>>>,
}

impl ExecutionEngine {
    /// Create a new ExecutionEngine with an optional MEV provider.
    /// The MEV provider can be a single provider (JitoClient, NextBlockClient, etc.)
    /// or a MevRouter that dispatches to multiple providers.
    pub fn new(
        rpc_http_url: String,
        keypair_path: String,
        mev_provider: Option<Arc<dyn MevProvider>>,
    ) -> Self {
        if let Some(ref provider) = mev_provider {
            info!("ExecutionEngine initialized with MEV provider: {}", provider.name());
        } else {
            info!("ExecutionEngine initialized without MEV provider (RPC-only mode)");
        }
        Self {
            rpc: Arc::new(RpcClient::new_with_commitment(
                rpc_http_url,
                CommitmentConfig::confirmed(),
            )),
            keypair_path,
            mev_provider,
            active_trades: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub fn rpc_client(&self) -> &RpcClient {
        &self.rpc
    }

    /// Cancel an in-flight trade by token mint address.
    pub async fn cancel_trade(&self, token_mint: &str) -> bool {
        let trades = self.active_trades.lock().await;
        if let Some(cancel_token) = trades.get(token_mint) {
            cancel_token.cancel();
            info!(token_mint, "Trade cancellation requested");
            true
        } else {
            warn!(token_mint, "No active trade found to cancel");
            false
        }
    }

    /// Execute a trade plan with abort support and retry logic.
    pub async fn execute(&self, plan: TradePlan) -> EngineResult<String> {
        let cancel_token = CancellationToken::new();
        let trade_key = format!("{:?}", plan.launchpad);

        {
            let mut trades = self.active_trades.lock().await;
            trades.insert(trade_key.clone(), cancel_token.clone());
        }

        let result = self.execute_inner(plan, &cancel_token).await;

        {
            let mut trades = self.active_trades.lock().await;
            trades.remove(&trade_key);
        }

        result
    }

    /// Inner execution with retry loop.
    async fn execute_inner(
        &self,
        plan: TradePlan,
        cancel_token: &CancellationToken,
    ) -> EngineResult<String> {
        let payer = self.load_keypair()?;

        if plan.simulate {
            return self.simulate_only(&plan, &payer).await;
        }

        let mut last_error = None;
        for attempt in 0..MAX_RETRIES {
            if cancel_token.is_cancelled() {
                return Err(EngineError::Cancelled("Trade cancelled by user".into()));
            }

            if attempt > 0 {
                let delay = RETRY_BASE_DELAY * 2u32.pow(attempt - 1);
                warn!(attempt, delay_ms = delay.as_millis(), "Retrying trade...");
                tokio::time::sleep(delay).await;
            }

            match self.try_execute(&plan, &payer, cancel_token).await {
                Ok(sig) => return Ok(sig),
                Err(e) if is_retryable(&e) => {
                    warn!(attempt, error = %e, "Transient failure, will retry");
                    last_error = Some(e);
                }
                Err(e) => return Err(e),
            }
        }

        Err(last_error.unwrap_or_else(|| {
            EngineError::Rpc("Max retries exhausted".into())
        }))
    }

    /// Single execution attempt: build tx → submit (parallel MEV + RPC) → confirm.
    async fn try_execute(
        &self,
        plan: &TradePlan,
        payer: &Keypair,
        cancel_token: &CancellationToken,
    ) -> EngineResult<String> {
        // Step 1: Fetch blockhash
        let bh = self.rpc.get_latest_blockhash().await
            .map_err(|e| EngineError::Rpc(format!("blockhash: {e}")))?;

        if cancel_token.is_cancelled() {
            return Err(EngineError::Cancelled("Trade cancelled by user".into()));
        }

        // Step 2: Build and sign transaction
        let mut tx = Transaction::new_with_payer(&plan.instructions, Some(&payer.pubkey()));
        tx.sign(&[payer], bh);
        let tx_sig = tx.signatures[0];

        info!(%tx_sig, "Transaction built and signed");

        if cancel_token.is_cancelled() {
            return Err(EngineError::Cancelled("Trade cancelled before submission".into()));
        }

        // Step 3: Parallel submission (MEV provider + RPC)
        self.submit_parallel(&tx, cancel_token).await?;

        // Step 4: Confirmation polling
        info!(%tx_sig, "Waiting for on-chain confirmation...");
        self.confirm_transaction(&tx_sig, cancel_token).await?;

        info!(%tx_sig, "✅ Transaction confirmed on-chain!");
        Ok(tx_sig.to_string())
    }

    /// Submit transaction via both MEV provider and RPC in parallel.
    async fn submit_parallel(
        &self,
        tx: &Transaction,
        cancel_token: &CancellationToken,
    ) -> EngineResult<()> {
        if let Some(ref mev) = self.mev_provider {
            let mev_tx = tx.clone();
            let mev_name = mev.name().to_string();
            let rpc_future = self.send_via_rpc(tx);
            let mev_future = mev.submit_bundle(vec![mev_tx]);

            info!("Submitting via {} + RPC in parallel...", mev_name);

            tokio::select! {
                _ = cancel_token.cancelled() => {
                    return Err(EngineError::Cancelled("Trade cancelled during submission".into()));
                }
                mev_result = mev_future => {
                    match mev_result {
                        Ok(bundle_id) => {
                            info!(%bundle_id, provider = %mev_name, "MEV bundle accepted (won race)");
                        }
                        Err(e) => {
                            warn!(provider = %mev_name, "MEV failed: {}. RPC continues.", e);
                        }
                    }
                }
                rpc_result = rpc_future => {
                    match rpc_result {
                        Ok(sig) => {
                            info!(%sig, "RPC submission succeeded (won race)");
                        }
                        Err(e) => {
                            warn!("RPC send failed: {}. MEV continues.", e);
                        }
                    }
                }
            }

            // Also fire the other path to maximize landing probability
            let _ = self.send_via_rpc(tx).await;
        } else {
            // No MEV provider — just RPC
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    return Err(EngineError::Cancelled("Trade cancelled during submission".into()));
                }
                result = self.send_via_rpc(tx) => {
                    result?;
                }
            }
        }

        Ok(())
    }

    /// Send via standard RPC (skip_preflight for speed).
    async fn send_via_rpc(&self, tx: &Transaction) -> EngineResult<Signature> {
        let config = solana_client::rpc_config::RpcSendTransactionConfig {
            skip_preflight: true,
            ..Default::default()
        };
        let sig = self.rpc.send_transaction_with_config(tx, config).await
            .map_err(|e| EngineError::Rpc(format!("send tx: {e}")))?;
        info!(%sig, "Submitted transaction via RPC");
        Ok(sig)
    }

    /// Simulate the transaction without sending it.
    async fn simulate_only(
        &self,
        plan: &TradePlan,
        payer: &Keypair,
    ) -> EngineResult<String> {
        let bh = self.rpc.get_latest_blockhash().await
            .map_err(|e| EngineError::Rpc(format!("blockhash: {e}")))?;

        let mut tx = Transaction::new_with_payer(&plan.instructions, Some(&payer.pubkey()));
        tx.sign(&[payer], bh);

        let sim = self.rpc.simulate_transaction(&tx).await
            .map_err(|e| EngineError::Rpc(format!("simulate rpc: {e}")))?;

        if let Some(err) = sim.value.err {
            if let Some(logs) = sim.value.logs {
                for log in &logs {
                    warn!("SIMULATION LOG: {}", log);
                }
            }
            return Err(EngineError::Simulation(format!("{err:?}")));
        }

        if let Some(logs) = sim.value.logs {
            for log in &logs {
                info!("SIM LOG: {}", log);
            }
        }

        info!("✅ Simulation successful");
        Ok("SIMULATED_SUCCESS".to_string())
    }

    /// Poll for transaction confirmation.
    async fn confirm_transaction(
        &self,
        sig: &Signature,
        cancel_token: &CancellationToken,
    ) -> EngineResult<()> {
        let start = Instant::now();

        loop {
            if cancel_token.is_cancelled() {
                return Err(EngineError::Cancelled(format!(
                    "Trade cancelled while waiting for confirmation of {}", sig
                )));
            }

            if start.elapsed() > CONFIRM_TIMEOUT {
                return Err(EngineError::Timeout(format!(
                    "Transaction {} not confirmed within {}s. Check: https://solscan.io/tx/{}",
                    sig, CONFIRM_TIMEOUT.as_secs(), sig
                )));
            }

            match self.rpc.get_signature_statuses(&[*sig]).await {
                Ok(response) => {
                    if let Some(Some(status)) = response.value.first() {
                        if let Some(ref err) = status.err {
                            return Err(EngineError::Confirmation(format!(
                                "Transaction {} FAILED on-chain: {:?}. View: https://solscan.io/tx/{}",
                                sig, err, sig
                            )));
                        }
                        return Ok(());
                    }
                }
                Err(e) => {
                    warn!("Error polling signature status: {e}");
                }
            }

            tokio::time::sleep(CONFIRM_POLL_INTERVAL).await;
        }
    }

    /// Load the keypair from file or base58 string.
    fn load_keypair(&self) -> EngineResult<Keypair> {
        let kp_path = shellexpand::tilde(&self.keypair_path).to_string();
        let keypair = read_keypair_file(&kp_path)
            .unwrap_or_else(|_| Keypair::from_base58_string(&self.keypair_path));
        Ok(keypair)
    }
}

// ── Error Classification ─────────────────────────────────────────────────────

fn is_retryable(err: &EngineError) -> bool {
    match err {
        EngineError::Rpc(msg) => {
            msg.contains("blockhash")
                || msg.contains("Blockhash not found")
                || msg.contains("connection")
                || msg.contains("timeout")
                || msg.contains("timed out")
                || msg.contains("503")
                || msg.contains("429")
        }
        EngineError::Timeout(_) => true,
        _ => false,
    }
}
