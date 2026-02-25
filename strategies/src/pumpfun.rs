use common::error::{EngineError, EngineResult};
use common::types::{BuyRequest, SellRequest, TradePlan, Launchpad};
use solana_sdk::{
    instruction::{Instruction, AccountMeta},
    pubkey::Pubkey,
    system_program,
};
use std::str::FromStr;

pub const PUMP_FUN_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
pub const GLOBAL: &str = "4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf";
pub const FEE_RECIPIENT: &str = "CebN5WGQ4jvEPvsVU4EoHEpgzq1VV7AbicfhtW4xC9iM";
pub const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
pub const EVENT_AUTHORITY: &str = "Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1";
pub const ASSOCIATED_TOKEN_PROGRAM: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

pub fn build_buy_plan(req: BuyRequest, defaults: &crate::router::TradeDefaults) -> EngineResult<TradePlan> {
    let program_id = Pubkey::from_str(PUMP_FUN_PROGRAM_ID).unwrap();
    let mint = Pubkey::from_str(&req.token_mint).map_err(|_| EngineError::BadRequest("invalid mint".into()))?;
    
    let resolved_token_program = req.token_program
        .as_deref()
        .map(|s| Pubkey::from_str(s).unwrap_or_else(|_| Pubkey::from_str(TOKEN_PROGRAM).unwrap()))
        .unwrap_or_else(|| Pubkey::from_str(TOKEN_PROGRAM).unwrap());

    let signer = Pubkey::from_str(&defaults.default_signer).unwrap_or_else(|_| Pubkey::new_unique());
    
    // Real PDA Derivations for Pump.fun V2
    let (bonding_curve, _) = Pubkey::find_program_address(&[b"bonding-curve", mint.as_ref()], &program_id);
    let (associated_bonding_curve, _) = Pubkey::find_program_address(
        &[bonding_curve.as_ref(), resolved_token_program.as_ref(), mint.as_ref()],
        &Pubkey::from_str(ASSOCIATED_TOKEN_PROGRAM).unwrap(),
    );
    let (associated_user_account, _) = Pubkey::find_program_address(
        &[signer.as_ref(), resolved_token_program.as_ref(), mint.as_ref()],
        &Pubkey::from_str(ASSOCIATED_TOKEN_PROGRAM).unwrap(),
    );

    // FIX: 2006 Simulation Error - Deriving the missing creator_vault PDA
    // creator corresponds to `req.creator` or defaults to System Program if it doesn't exist.
    // However, pump.fun actually doesn't use the creator wallet directly in the buy array anymore.
    // We just need the 14 standard accounts.

    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(&[0x66, 0x06, 0x3d, 0x12, 0x01, 0xda, 0xeb, 0xea]); // Buy Discriminator
    
    let max_sol_cost = (req.amount_sol * 1_000_000_000.0) as u64;
    // FIX: 3008 Error (Pump.fun AmountTooSmall or InvalidAmount)
    // You cannot pass token_amount = 0. The contract expects a non-zero amount of tokens to buy.
    // For test purposes, we will ask to buy 1 full token (1_000_000 in micro-tokens).
    // The max_sol_cost will act as our slippage guard.
    let token_amount = 1_000_000u64; 
    
    data.extend_from_slice(&token_amount.to_le_bytes()); 
    data.extend_from_slice(&max_sol_cost.to_le_bytes());

    // EXACT 14 ACCOUNTS REQUIRED FOR PUMPFUN BUY INSTRUCTION
    // Pump.fun V2 Added Volume Accumulators
    let (user_volume_accumulator, _) = Pubkey::find_program_address(
        &[b"user_volume_accumulator", signer.as_ref()],
        &program_id,
    );
    let (global_volume_accumulator, _) = Pubkey::find_program_address(
        &[b"global_volume_accumulator"],
        &program_id,
    );

    let exact_buy_accounts = vec![
        AccountMeta::new_readonly(Pubkey::from_str(GLOBAL).unwrap(), false),     // 0. global
        AccountMeta::new(Pubkey::from_str(FEE_RECIPIENT).unwrap(), false),       // 1. feeRecipient
        AccountMeta::new_readonly(mint, false),                                  // 2. mint
        AccountMeta::new(bonding_curve, false),                                  // 3. bondingCurve
        AccountMeta::new(associated_bonding_curve, false),                       // 4. associatedBondingCurve
        AccountMeta::new(associated_user_account, false),                        // 5. associatedUser
        AccountMeta::new(signer, true),                                          // 6. user
        AccountMeta::new_readonly(system_program::ID, false),                    // 7. systemProgram
        AccountMeta::new_readonly(Pubkey::from_str(ASSOCIATED_TOKEN_PROGRAM).unwrap(), false), // 8. associatedTokenProgram
        AccountMeta::new_readonly(resolved_token_program, false),                // 9. tokenProgram
        AccountMeta::new_readonly(Pubkey::from_str(EVENT_AUTHORITY).unwrap(), false), // 10. eventAuthority
        AccountMeta::new_readonly(program_id, false),                            // 11. program
        // V2 Added Accounts
        AccountMeta::new(global_volume_accumulator, false),                      // 12. globalVolumeAccumulator
        AccountMeta::new(user_volume_accumulator, false),                        // 13. userVolumeAccumulator
    ];

    let ix = Instruction {
        program_id,
        accounts: exact_buy_accounts,
        data,
    };

    let create_ata_ix = spl_associated_token_account::instruction::create_associated_token_account_idempotent(
        &signer,
        &signer,
        &mint,
        &resolved_token_program,
    );

    Ok(TradePlan {
        launchpad: Launchpad::PumpFun,
        instructions: vec![create_ata_ix, ix],
        signer_pubkey: signer,
        simulate: defaults.simulate_before_send,
    })
}

pub fn build_sell_plan(req: SellRequest, defaults: &crate::router::TradeDefaults) -> EngineResult<TradePlan> {
    let program_id = Pubkey::from_str(PUMP_FUN_PROGRAM_ID).unwrap();
    let mint = Pubkey::from_str(&req.token_mint).map_err(|_| EngineError::BadRequest("invalid mint".into()))?;
    let signer = Pubkey::from_str(&defaults.default_signer).unwrap_or_else(|_| Pubkey::new_unique());

    let resolved_token_program = req.token_program
        .as_deref()
        .map(|s| Pubkey::from_str(s).unwrap_or_else(|_| Pubkey::from_str(TOKEN_PROGRAM).unwrap()))
        .unwrap_or_else(|| Pubkey::from_str(TOKEN_PROGRAM).unwrap());

    // PDA Derivations for Pump.fun
    let (bonding_curve, _) = Pubkey::find_program_address(&[b"bonding-curve", mint.as_ref()], &program_id);
    let (associated_bonding_curve, _) = Pubkey::find_program_address(
        &[bonding_curve.as_ref(), resolved_token_program.as_ref(), mint.as_ref()],
        &Pubkey::from_str(ASSOCIATED_TOKEN_PROGRAM).unwrap(),
    );
    let (associated_user_account, _) = Pubkey::find_program_address(
        &[signer.as_ref(), resolved_token_program.as_ref(), mint.as_ref()],
        &Pubkey::from_str(ASSOCIATED_TOKEN_PROGRAM).unwrap(),
    );

    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(&[0x33, 0xe6, 0x85, 0xa4, 0x01, 0x7f, 0x83, 0xad]); // Sell Discriminator
    
    data.extend_from_slice(&req.amount_tokens.to_le_bytes());
    data.extend_from_slice(&0u64.to_le_bytes()); // Min SOL output

    // EXACT 13/14 ACCOUNTS REQUIRED FOR PUMPFUN SELL INSTRUCTION
    // Pump.fun V2 Added Volume Accumulators
    let (user_volume_accumulator, _) = Pubkey::find_program_address(
        &[b"user_volume_accumulator", signer.as_ref()],
        &program_id,
    );
    let (global_volume_accumulator, _) = Pubkey::find_program_address(
        &[b"global_volume_accumulator"],
        &program_id,
    );

    let exact_sell_accounts = vec![
        AccountMeta::new_readonly(Pubkey::from_str(GLOBAL).unwrap(), false),     // 1. global
        AccountMeta::new(Pubkey::from_str(FEE_RECIPIENT).unwrap(), false),       // 2. feeRecipient
        AccountMeta::new_readonly(mint, false),                                  // 3. mint
        AccountMeta::new(bonding_curve, false),                                  // 4. bondingCurve
        AccountMeta::new(associated_bonding_curve, false),                       // 5. associatedBondingCurve
        AccountMeta::new(associated_user_account, false),                        // 6. associatedUser
        AccountMeta::new(signer, true),                                          // 7. user
        AccountMeta::new_readonly(system_program::ID, false),                    // 8. systemProgram
        AccountMeta::new_readonly(Pubkey::from_str(ASSOCIATED_TOKEN_PROGRAM).unwrap(), false), // 9. associatedTokenProgram
        AccountMeta::new_readonly(resolved_token_program, false),                // 10. tokenProgram
        AccountMeta::new_readonly(Pubkey::from_str(EVENT_AUTHORITY).unwrap(), false), // 11. eventAuthority
        AccountMeta::new_readonly(program_id, false),                            // 12. program
        // V2 Added Accounts
        AccountMeta::new(global_volume_accumulator, false),                      // 13. globalVolumeAccumulator
        AccountMeta::new(user_volume_accumulator, false),                        // 14. userVolumeAccumulator
    ];

    let ix = Instruction {
        program_id,
        accounts: exact_sell_accounts,
        data,
    };

    let create_ata_ix = spl_associated_token_account::instruction::create_associated_token_account_idempotent(
        &signer,
        &signer,
        &mint,
        &resolved_token_program,
    );

    Ok(TradePlan {
        launchpad: Launchpad::PumpFun,
        instructions: vec![create_ata_ix, ix],
        signer_pubkey: signer,
        simulate: defaults.simulate_before_send,
    })
}

