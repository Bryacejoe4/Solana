/// MevRouter — smart MEV provider selection and submission.
///
/// Strategies:
///   - `Parallel`: Submit to ALL providers simultaneously, return first success
///   - `RoundRobin`: Rotate through providers sequentially
///   - `Fastest`: Use the provider that responded fastest historically (TODO)

use crate::MevProvider;
use async_trait::async_trait;
use solana_sdk::transaction::Transaction;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tracing::{info, warn, error};

/// Routing strategy for MEV bundle submission.
#[derive(Debug, Clone, PartialEq)]
pub enum MevStrategy {
    /// Submit to all providers simultaneously, first success wins.
    Parallel,
    /// Rotate through providers one at a time.
    RoundRobin,
}

pub struct MevRouter {
    providers: Vec<Arc<dyn MevProvider>>,
    strategy: MevStrategy,
    round_robin_index: AtomicUsize,
}

impl MevRouter {
    pub fn new(providers: Vec<Arc<dyn MevProvider>>, strategy: MevStrategy) -> Self {
        assert!(!providers.is_empty(), "MevRouter requires at least one provider");
        info!(
            "MevRouter initialized with {} providers: [{}], strategy: {:?}",
            providers.len(),
            providers.iter().map(|p| p.name()).collect::<Vec<_>>().join(", "),
            strategy,
        );
        Self {
            providers,
            strategy,
            round_robin_index: AtomicUsize::new(0),
        }
    }

    /// Submit a bundle using the configured strategy.
    pub async fn submit_bundle(&self, txs: Vec<Transaction>) -> Result<String, String> {
        match self.strategy {
            MevStrategy::Parallel => self.submit_parallel(txs).await,
            MevStrategy::RoundRobin => self.submit_round_robin(txs).await,
        }
    }

    /// Submit to ALL providers in parallel.
    /// Returns the first successful result, or all errors if all fail.
    async fn submit_parallel(&self, txs: Vec<Transaction>) -> Result<String, String> {
        use tokio::task::JoinSet;

        let mut join_set = JoinSet::new();
        let mut errors = Vec::new();

        for provider in &self.providers {
            let provider = Arc::clone(provider);
            let txs = txs.clone();
            join_set.spawn(async move {
                let name = provider.name().to_string();
                let result = provider.submit_bundle(txs).await;
                (name, result)
            });
        }

        // Collect results as they complete
        while let Some(result) = join_set.join_next().await {
            match result {
                Ok((name, Ok(bundle_id))) => {
                    info!("✅ {} won the race — bundle: {}", name, bundle_id);
                    // Abort remaining tasks (they'll be dropped)
                    join_set.abort_all();
                    return Ok(bundle_id);
                }
                Ok((name, Err(e))) => {
                    warn!("❌ {} failed: {}", name, e);
                    errors.push(format!("{}: {}", name, e));
                }
                Err(e) => {
                    error!("Provider task panicked: {}", e);
                    errors.push(format!("panic: {}", e));
                }
            }
        }

        Err(format!("All providers failed: {}", errors.join("; ")))
    }

    /// Submit to one provider at a time, rotating through them.
    /// Falls back to the next provider if the current one fails.
    async fn submit_round_robin(&self, txs: Vec<Transaction>) -> Result<String, String> {
        let start_index = self.round_robin_index.fetch_add(1, Ordering::Relaxed) % self.providers.len();
        let mut errors = Vec::new();

        for i in 0..self.providers.len() {
            let idx = (start_index + i) % self.providers.len();
            let provider = &self.providers[idx];
            let name = provider.name();

            info!("Trying provider {} ({}/{})", name, i + 1, self.providers.len());

            match provider.submit_bundle(txs.clone()).await {
                Ok(bundle_id) => {
                    info!("✅ {} accepted bundle: {}", name, bundle_id);
                    return Ok(bundle_id);
                }
                Err(e) => {
                    warn!("❌ {} failed: {}. Trying next...", name, e);
                    errors.push(format!("{}: {}", name, e));
                }
            }
        }

        Err(format!("All providers failed (round-robin): {}", errors.join("; ")))
    }

    /// Get the number of configured providers.
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }
}

/// Implement MevProvider for MevRouter so it can be used anywhere a single provider is expected.
#[async_trait]
impl MevProvider for MevRouter {
    async fn submit_bundle(&self, txs: Vec<Transaction>) -> Result<String, String> {
        self.submit_bundle(txs).await
    }

    fn name(&self) -> &str {
        "MevRouter"
    }
}
