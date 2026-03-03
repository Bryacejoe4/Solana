/// Dynamic priority fee calculation.
///
/// Queries recent prioritization fees for the target program and
/// computes optimal ComputeBudget instructions to include in transactions.

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    instruction::Instruction,
    pubkey::Pubkey,
};
use tracing::{info, warn};

/// Default compute unit limit for Pump.fun transactions.
const DEFAULT_CU_LIMIT: u32 = 200_000;

/// Default priority fee if we can't fetch recent fees (in microLamports).
const DEFAULT_PRIORITY_FEE: u64 = 10_000;

/// Multiplier applied to the max recent fee for competitive priority.
const FEE_MULTIPLIER: f64 = 1.2;

/// Build ComputeBudget instructions with dynamic priority fees.
///
/// Queries `getRecentPrioritizationFees` for the given program and sets
/// the priority fee to 1.2× the recent maximum, or a sensible default.
pub async fn build_priority_instructions(
    rpc: &RpcClient,
    program_id: &Pubkey,
    cu_limit: Option<u32>,
) -> Vec<Instruction> {
    let limit = cu_limit.unwrap_or(DEFAULT_CU_LIMIT);
    let mut ixs = vec![ComputeBudgetInstruction::set_compute_unit_limit(limit)];

    let fee = match fetch_dynamic_fee(rpc, program_id).await {
        Ok(f) => f,
        Err(e) => {
            warn!("Failed to fetch priority fees, using default: {}", e);
            DEFAULT_PRIORITY_FEE
        }
    };

    info!(
        priority_fee_micro_lamports = fee,
        cu_limit = limit,
        "Priority fee calculated"
    );

    ixs.push(ComputeBudgetInstruction::set_compute_unit_price(fee));
    ixs
}

/// Build priority instructions with a fixed fee (no RPC query).
pub fn build_fixed_priority_instructions(
    priority_fee_micro_lamports: u64,
    cu_limit: Option<u32>,
) -> Vec<Instruction> {
    let limit = cu_limit.unwrap_or(DEFAULT_CU_LIMIT);
    vec![
        ComputeBudgetInstruction::set_compute_unit_limit(limit),
        ComputeBudgetInstruction::set_compute_unit_price(priority_fee_micro_lamports),
    ]
}

/// Fetch the optimal priority fee by querying recent prioritization fees.
async fn fetch_dynamic_fee(rpc: &RpcClient, program_id: &Pubkey) -> Result<u64, String> {
    let recent_fees = rpc
        .get_recent_prioritization_fees(&[*program_id])
        .await
        .map_err(|e| format!("getRecentPrioritizationFees failed: {}", e))?;

    if recent_fees.is_empty() {
        return Ok(DEFAULT_PRIORITY_FEE);
    }

    let max_fee = recent_fees
        .iter()
        .map(|f| f.prioritization_fee)
        .max()
        .unwrap_or(0);

    if max_fee == 0 {
        return Ok(DEFAULT_PRIORITY_FEE);
    }

    let optimized = (max_fee as f64 * FEE_MULTIPLIER).ceil() as u64;
    Ok(optimized)
}
