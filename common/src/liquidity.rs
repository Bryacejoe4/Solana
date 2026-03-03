/// On-chain liquidity checking for trade safety.
///
/// Before executing a trade, we can check the bonding curve's reserves
/// to ensure there's enough liquidity to fill the order without
/// excessive slippage or a rug.

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use tracing::{info, warn};

use crate::curve;

/// Check if a bonding curve has at minimum the specified SOL liquidity.
///
/// Returns `Ok(true)` if liquidity is sufficient, `Ok(false)` if not,
/// or `Err` if the account cannot be fetched.
pub async fn check_liquidity(
    rpc: &RpcClient,
    bonding_curve: &Pubkey,
    min_sol_lamports: u64,
) -> Result<bool, String> {
    let account = rpc
        .get_account(bonding_curve)
        .await
        .map_err(|e| format!("Failed to fetch bonding curve {}: {}", bonding_curve, e))?;

    let data = &account.data;

    // Check if curve has graduated (complete)
    if curve::is_curve_complete(data) {
        warn!(%bonding_curve, "Bonding curve is complete (token graduated)");
        return Ok(false);
    }

    let virtual_sol = curve::get_virtual_sol_reserves(data);
    let real_sol = curve::get_real_sol_reserves(data);

    info!(
        %bonding_curve,
        virtual_sol_lamports = virtual_sol,
        real_sol_lamports = real_sol,
        min_required = min_sol_lamports,
        "Liquidity check"
    );

    if real_sol < min_sol_lamports {
        warn!(
            %bonding_curve,
            real_sol = real_sol,
            min_required = min_sol_lamports,
            "Insufficient liquidity — below minimum"
        );
        return Ok(false);
    }

    Ok(true)
}

/// Fetch the bonding curve data and compute expected tokens out for a given SOL input.
/// Returns `(tokens_out, is_curve_complete)`.
pub async fn preview_buy(
    rpc: &RpcClient,
    bonding_curve: &Pubkey,
    sol_amount_lamports: u64,
) -> Result<(u64, bool), String> {
    let account = rpc
        .get_account(bonding_curve)
        .await
        .map_err(|e| format!("Failed to fetch bonding curve {}: {}", bonding_curve, e))?;

    let data = &account.data;
    let complete = curve::is_curve_complete(data);
    let tokens_out = curve::get_tokens_out(data, sol_amount_lamports);

    Ok((tokens_out, complete))
}

/// Fetch the bonding curve data and compute expected SOL out for a given token sell.
/// Returns `(sol_out, is_curve_complete)`.
pub async fn preview_sell(
    rpc: &RpcClient,
    bonding_curve: &Pubkey,
    token_amount: u64,
) -> Result<(u64, bool), String> {
    let account = rpc
        .get_account(bonding_curve)
        .await
        .map_err(|e| format!("Failed to fetch bonding curve {}: {}", bonding_curve, e))?;

    let data = &account.data;
    let complete = curve::is_curve_complete(data);
    let sol_out = curve::get_sol_out(data, token_amount);

    Ok((sol_out, complete))
}
