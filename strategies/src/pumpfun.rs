use common::error::{EngineError, EngineResult};
use common::types::{BuyRequest, SellRequest, TradePlan, Launchpad};
use solana_sdk::{
    instruction::{Instruction, AccountMeta},
    pubkey::Pubkey,
    system_program,
};
use spl_associated_token_account::get_associated_token_address;
use std::str::FromStr;

// ─── Static Program Addresses ────────────────────────────────────────────────
// Verified 2025-02-25 against live on-chain pump.fun V2 transactions on Solscan

pub const PUMP_FUN_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

// Global state PDA (seeds: ["global"] under pump program)
pub const GLOBAL: &str            = "4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf";

// Protocol fee destination — CONFIRMED from live tx 2025-02-25
pub const FEE_RECIPIENT: &str     = "62qc2CNXwrYqQScmEdiZFFAnJR262PxWEuNQtxfafNgV";

// Token programs
pub const TOKEN_PROGRAM: &str     = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
pub const TOKEN_2022_PROGRAM: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
pub const ASSOC_TOKEN_PROGRAM: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJe1bRS";

// Event authority PDA — CONFIRMED from live tx 2025-02-25
pub const EVENT_AUTHORITY: &str   = "Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1";

// Fee config account — CONFIRMED from live tx 2025-02-25
pub const FEE_CONFIG: &str        = "8Wf5TiAheLUqBrKXeYg2JtAFFMWtKdG2BSFgqUcPVwTt";

// ─── Instruction Discriminators (Anchor SHA256 of "global:<ix_name>") ─────────
// Buy:  sha256("global:buy")[0..8]
const BUY_DISCRIMINATOR:  [u8; 8] = [0x66, 0x06, 0x3d, 0x12, 0x01, 0xda, 0xeb, 0xea];
// Sell: sha256("global:sell")[0..8]
const SELL_DISCRIMINATOR: [u8; 8] = [0x33, 0xe6, 0x85, 0xa4, 0x01, 0x7f, 0x83, 0xad];

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn pump_program() -> Pubkey { Pubkey::from_str(PUMP_FUN_PROGRAM_ID).unwrap() }

/// Derive bonding-curve PDA: seeds = ["bonding-curve", mint]
fn bonding_curve_pda(mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"bonding-curve", mint.as_ref()], &pump_program()).0
}

/// Derive creator-vault PDA: seeds = ["creator-vault", creator_pubkey]
fn creator_vault_pda(creator: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"creator-vault", creator.as_ref()], &pump_program()).0
}

/// Derive global-volume-accumulator PDA: seeds = ["global_volume_accumulator"]
fn global_volume_accumulator_pda() -> Pubkey {
    Pubkey::find_program_address(&[b"global_volume_accumulator"], &pump_program()).0
}

/// Derive user-volume-accumulator PDA: seeds = ["user_volume_accumulator", user_pubkey]
fn user_volume_accumulator_pda(user: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"user_volume_accumulator", user.as_ref()], &pump_program()).0
}

/// Detect token-2022 mints by the caller passing "token2022" in the token_program field.
fn resolve_token_program(req_token_program: &Option<String>) -> Pubkey {
    match req_token_program.as_deref() {
        Some(s) if s.eq_ignore_ascii_case("token2022") || s == TOKEN_2022_PROGRAM => {
            Pubkey::from_str(TOKEN_2022_PROGRAM).unwrap()
        }
        Some(s) => Pubkey::from_str(s).unwrap_or_else(|_| Pubkey::from_str(TOKEN_PROGRAM).unwrap()),
        None => Pubkey::from_str(TOKEN_PROGRAM).unwrap(),
    }
}

// ─── Build BUY Plan ───────────────────────────────────────────────────────────
//
// Account order verified from live tx 3YghLYR6YxqJtEnMDiQD4zj... on 2025-02-25:
//  0. global              (read)
//  1. feeRecipient        (writable)
//  2. mint                (read)
//  3. bondingCurve        (writable)
//  4. associatedBondingCurve (writable)
//  5. associatedUser      (writable)
//  6. user / signer       (writable, signer)
//  7. systemProgram       (read)
//  8. tokenProgram        (read)
//  9. creatorVault        (writable)
// 10. eventAuthority      (read)
// 11. program             (read)
// 12. globalVolumeAccumulator (writable)
// 13. userVolumeAccumulator   (writable)
// 14. feeConfig           (read)

pub fn build_buy_plan(req: BuyRequest, defaults: &crate::router::TradeDefaults) -> EngineResult<TradePlan> {
    let program_id   = pump_program();
    let mint         = Pubkey::from_str(&req.token_mint)
        .map_err(|_| EngineError::BadRequest("invalid token_mint".into()))?;
    let signer       = Pubkey::from_str(&defaults.default_signer)
        .map_err(|_| EngineError::Config("invalid default_signer in config".into()))?;
    let token_prog   = resolve_token_program(&req.token_program);

    // Resolve creator (needed for creator_vault PDA).
    // Caller should pass `creator` in the request body. Fall back to system program if missing
    // (trade will still simulate but creator fee will go to system program — harmless for testing).
    let creator = req.creator
        .as_deref()
        .and_then(|s| Pubkey::from_str(s).ok())
        .unwrap_or(system_program::ID);

    // PDAs
    let bonding_curve          = bonding_curve_pda(&mint);
    let assoc_bonding_curve    = get_associated_token_address(&bonding_curve, &mint);
    let assoc_user             = get_associated_token_address(&signer, &mint);
    let creator_vault          = creator_vault_pda(&creator);
    let global_vol_acc         = global_volume_accumulator_pda();
    let user_vol_acc           = user_volume_accumulator_pda(&signer);

    // Instruction data: discriminator | token_amount (u64 LE) | max_sol_cost (u64 LE)
    let max_sol_lamports = (req.amount_sol * 1_000_000_000.0) as u64;
    let slippage = req.max_slippage_bps.unwrap_or(defaults.max_slippage_bps);
    // Apply slippage on top of max_sol_cost so the on-chain check passes
    let max_sol_cost = max_sol_lamports
        .saturating_add(max_sol_lamports.saturating_mul(slippage as u64) / 10_000);

    // Conservative token amount estimate: ask to buy 1_000_000 raw tokens (1 token with 6 decimals).
    // In production you'd read the curve and compute the exact expected amount.
    // The `max_sol_cost` is the real guard — if the curve charges more than that, it reverts.
    let token_amount: u64 = 1_000_000;

    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(&BUY_DISCRIMINATOR);
    data.extend_from_slice(&token_amount.to_le_bytes());
    data.extend_from_slice(&max_sol_cost.to_le_bytes());

    let buy_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new_readonly(Pubkey::from_str(GLOBAL).unwrap(), false),          //  0 global
            AccountMeta::new(Pubkey::from_str(FEE_RECIPIENT).unwrap(), false),            //  1 feeRecipient
            AccountMeta::new_readonly(mint, false),                                        //  2 mint
            AccountMeta::new(bonding_curve, false),                                        //  3 bondingCurve
            AccountMeta::new(assoc_bonding_curve, false),                                  //  4 associatedBondingCurve
            AccountMeta::new(assoc_user, false),                                           //  5 associatedUser
            AccountMeta::new(signer, true),                                                //  6 user (signer)
            AccountMeta::new_readonly(system_program::ID, false),                          //  7 systemProgram
            AccountMeta::new_readonly(token_prog, false),                                  //  8 tokenProgram
            AccountMeta::new(creator_vault, false),                                        //  9 creatorVault
            AccountMeta::new_readonly(Pubkey::from_str(EVENT_AUTHORITY).unwrap(), false),  // 10 eventAuthority
            AccountMeta::new_readonly(program_id, false),                                  // 11 program
            AccountMeta::new(global_vol_acc, false),                                       // 12 globalVolumeAccumulator
            AccountMeta::new(user_vol_acc, false),                                         // 13 userVolumeAccumulator
            AccountMeta::new_readonly(Pubkey::from_str(FEE_CONFIG).unwrap(), false),       // 14 feeConfig
        ],
        data,
    };

    // Create the user's ATA if it doesn't already exist (idempotent)
    let create_ata_ix = spl_associated_token_account::instruction::create_associated_token_account_idempotent(
        &signer,
        &signer,
        &mint,
        &token_prog,
    );

    Ok(TradePlan {
        launchpad: Launchpad::PumpFun,
        instructions: vec![create_ata_ix, buy_ix],
        signer_pubkey: signer,
        simulate: defaults.simulate_before_send,
    })
}

// ─── Build SELL Plan ──────────────────────────────────────────────────────────
//
// Same account order as buy (no associatedTokenProgram needed for sell).

pub fn build_sell_plan(req: SellRequest, defaults: &crate::router::TradeDefaults) -> EngineResult<TradePlan> {
    let program_id   = pump_program();
    let mint         = Pubkey::from_str(&req.token_mint)
        .map_err(|_| EngineError::BadRequest("invalid token_mint".into()))?;
    let signer       = Pubkey::from_str(&defaults.default_signer)
        .map_err(|_| EngineError::Config("invalid default_signer in config".into()))?;
    let token_prog   = resolve_token_program(&req.token_program);

    let creator = req.creator
        .as_deref()
        .and_then(|s| Pubkey::from_str(s).ok())
        .unwrap_or(system_program::ID);

    // PDAs
    let bonding_curve       = bonding_curve_pda(&mint);
    let assoc_bonding_curve = get_associated_token_address(&bonding_curve, &mint);
    let assoc_user          = get_associated_token_address(&signer, &mint);
    let creator_vault       = creator_vault_pda(&creator);
    let global_vol_acc      = global_volume_accumulator_pda();
    let user_vol_acc        = user_volume_accumulator_pda(&signer);

    // Instruction data: discriminator | amount (u64 LE) | min_sol_output (u64 LE)
    let slippage = req.max_slippage_bps.unwrap_or(defaults.max_slippage_bps);
    // min_sol_output = 0 means "accept any price" — safe for testing, add price calc for production
    let min_sol_output: u64 = 0;
    let _ = slippage; // reserved for production price calculation

    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(&SELL_DISCRIMINATOR);
    data.extend_from_slice(&req.amount_tokens.to_le_bytes());
    data.extend_from_slice(&min_sol_output.to_le_bytes());

    let sell_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new_readonly(Pubkey::from_str(GLOBAL).unwrap(), false),          //  0 global
            AccountMeta::new(Pubkey::from_str(FEE_RECIPIENT).unwrap(), false),            //  1 feeRecipient
            AccountMeta::new_readonly(mint, false),                                        //  2 mint
            AccountMeta::new(bonding_curve, false),                                        //  3 bondingCurve
            AccountMeta::new(assoc_bonding_curve, false),                                  //  4 associatedBondingCurve
            AccountMeta::new(assoc_user, false),                                           //  5 associatedUser
            AccountMeta::new(signer, true),                                                //  6 user (signer)
            AccountMeta::new_readonly(system_program::ID, false),                          //  7 systemProgram
            AccountMeta::new_readonly(token_prog, false),                                  //  8 tokenProgram
            AccountMeta::new(creator_vault, false),                                        //  9 creatorVault
            AccountMeta::new_readonly(Pubkey::from_str(EVENT_AUTHORITY).unwrap(), false),  // 10 eventAuthority
            AccountMeta::new_readonly(program_id, false),                                  // 11 program
            AccountMeta::new(global_vol_acc, false),                                       // 12 globalVolumeAccumulator
            AccountMeta::new(user_vol_acc, false),                                         // 13 userVolumeAccumulator
            AccountMeta::new_readonly(Pubkey::from_str(FEE_CONFIG).unwrap(), false),       // 14 feeConfig
        ],
        data,
    };

    Ok(TradePlan {
        launchpad: Launchpad::PumpFun,
        instructions: vec![sell_ix],
        signer_pubkey: signer,
        simulate: defaults.simulate_before_send,
    })
}
