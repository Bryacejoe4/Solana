/// Bonding curve math for Pump.fun AMM pricing.
///
/// Layout of the bonding-curve account (after 8-byte Anchor discriminator):
///   offset 8:  virtualTokenReserves  (u64, LE)
///   offset 16: virtualSolReserves    (u64, LE)
///   offset 24: realTokenReserves     (u64, LE)
///   offset 32: realSolReserves       (u64, LE)
///   offset 40: tokenTotalSupply      (u64, LE)
///   offset 48: complete              (bool, 1 byte)

/// Extract the current price (SOL per token) from raw bonding-curve account data.
pub fn get_price_from_data(data: &[u8]) -> f64 {
    if data.len() < 24 {
        return 0.0;
    }
    let virtual_token = u64::from_le_bytes(data[8..16].try_into().unwrap_or_default());
    let virtual_sol = u64::from_le_bytes(data[16..24].try_into().unwrap_or_default());

    if virtual_token == 0 {
        return 0.0;
    }
    virtual_sol as f64 / virtual_token as f64
}

/// Get the virtual SOL reserves from raw bonding-curve account data (in lamports).
pub fn get_virtual_sol_reserves(data: &[u8]) -> u64 {
    if data.len() < 24 {
        return 0;
    }
    u64::from_le_bytes(data[16..24].try_into().unwrap_or_default())
}

/// Get the real SOL reserves from raw bonding-curve account data (in lamports).
pub fn get_real_sol_reserves(data: &[u8]) -> u64 {
    if data.len() < 40 {
        return 0;
    }
    u64::from_le_bytes(data[32..40].try_into().unwrap_or_default())
}

/// Check if the bonding curve has completed (token graduated).
pub fn is_curve_complete(data: &[u8]) -> bool {
    if data.len() < 49 {
        return false;
    }
    data[48] != 0
}

/// Calculate the token amount out for a given SOL input using the constant-product formula.
///
/// Formula: amountOut = x - (x * y) / (y + dy)
///   where x = virtualTokenReserves, y = virtualSolReserves, dy = solAmountIn
pub fn get_tokens_out(data: &[u8], sol_amount_lamports: u64) -> u64 {
    if data.len() < 24 || sol_amount_lamports == 0 {
        return 0;
    }

    let x = u64::from_le_bytes(data[8..16].try_into().unwrap_or_default()) as u128;
    let y = u64::from_le_bytes(data[16..24].try_into().unwrap_or_default()) as u128;
    let dy = sol_amount_lamports as u128;

    if y + dy == 0 {
        return 0;
    }

    let new_x = (x * y) / (y + dy);
    let amount_out = x.saturating_sub(new_x);

    amount_out as u64
}

/// Calculate the SOL amount out for a given token input (for sells).
///
/// Formula: amountOut = y - (x * y) / (x + dx)
///   where x = virtualTokenReserves, y = virtualSolReserves, dx = tokenAmountIn
pub fn get_sol_out(data: &[u8], token_amount: u64) -> u64 {
    if data.len() < 24 || token_amount == 0 {
        return 0;
    }

    let x = u64::from_le_bytes(data[8..16].try_into().unwrap_or_default()) as u128;
    let y = u64::from_le_bytes(data[16..24].try_into().unwrap_or_default()) as u128;
    let dx = token_amount as u128;

    if x + dx == 0 {
        return 0;
    }

    let new_y = (x * y) / (x + dx);
    let amount_out = y.saturating_sub(new_y);

    amount_out as u64
}

/// Calculate the maximum SOL to pay (with slippage) for a buy.
pub fn max_sol_cost(data: &[u8], sol_amount_lamports: u64, slippage_bps: u16) -> u64 {
    let base = sol_amount_lamports as u128;
    let slippage = (base * slippage_bps as u128) / 10_000;
    (base + slippage) as u64
}

/// Calculate the minimum SOL to receive (with slippage) for a sell.
pub fn min_sol_output(expected_sol: u64, slippage_bps: u16) -> u64 {
    let base = expected_sol as u128;
    let slippage = (base * slippage_bps as u128) / 10_000;
    base.saturating_sub(slippage) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_curve_data(virtual_token: u64, virtual_sol: u64) -> Vec<u8> {
        let mut data = vec![0u8; 49];
        data[8..16].copy_from_slice(&virtual_token.to_le_bytes());
        data[16..24].copy_from_slice(&virtual_sol.to_le_bytes());
        data
    }

    #[test]
    fn test_get_price() {
        let data = make_curve_data(1_000_000_000, 30_000_000_000); // 1B tokens, 30 SOL
        let price = get_price_from_data(&data);
        assert!((price - 30.0).abs() < 0.0001);
    }

    #[test]
    fn test_get_price_zero_tokens() {
        let data = make_curve_data(0, 30_000_000_000);
        assert_eq!(get_price_from_data(&data), 0.0);
    }

    #[test]
    fn test_tokens_out() {
        // With 1B tokens and 30 SOL, buying with 1 SOL should give ~32M tokens
        let data = make_curve_data(1_000_000_000_000, 30_000_000_000);
        let out = get_tokens_out(&data, 1_000_000_000); // 1 SOL in lamports
        assert!(out > 0);
        // Rough check: should be around 1/31 of the token pool
        let expected_approx = 1_000_000_000_000u64 / 31;
        let diff = (out as i128 - expected_approx as i128).unsigned_abs();
        assert!(diff < expected_approx as u128 / 10); // within 10%
    }

    #[test]
    fn test_sol_out() {
        let data = make_curve_data(1_000_000_000_000, 30_000_000_000);
        let out = get_sol_out(&data, 100_000_000); // sell 100M tokens
        assert!(out > 0);
    }

    #[test]
    fn test_slippage() {
        let max = max_sol_cost(&[], 1_000_000_000, 500); // 5% slippage
        assert_eq!(max, 1_050_000_000);

        let min = min_sol_output(1_000_000_000, 500);
        assert_eq!(min, 950_000_000);
    }

    #[test]
    fn test_curve_complete() {
        let mut data = vec![0u8; 49];
        assert!(!is_curve_complete(&data));
        data[48] = 1;
        assert!(is_curve_complete(&data));
    }
}
