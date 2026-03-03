
pub mod jito;
pub mod nextblock;
pub mod zeroblock;
pub mod router;

use async_trait::async_trait;
use solana_sdk::transaction::Transaction;

/// Trait for MEV bundle submission providers.
///
/// All providers accept a vector of signed transactions and submit
/// them as a bundle to their respective block engine.
#[async_trait]
pub trait MevProvider: Send + Sync {
    /// Submit a bundle of transactions. Returns a bundle ID on success.
    async fn submit_bundle(&self, txs: Vec<Transaction>) -> Result<String, String>;

    /// Human-readable provider name (for logging).
    fn name(&self) -> &str;
}
