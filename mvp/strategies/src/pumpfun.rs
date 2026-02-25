use common::error::{EngineError, EngineResult};
use common::types::{BuyRequest, SellRequest, TradePlan, Launchpad};
use solana_sdk::{
    instruction::{Instruction, AccountMeta},
    pubkey::Pubkey,
    system_program,
    sysvar::rent,
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
    
    // In a real bot, we fetch these PDAs. Here we use placeholders.
    let signer = Pubkey::from_str(&defaults.default_signer).unwrap_or_else(|_| Pubkey::new_unique());
    
    // Real PDA Derivations for Pump.fun
    let (bonding_curve, _) = Pubkey::find_program_address(&[b"bonding-curve", mint.as_ref()], &program_id);
    let (associated_bonding_curve, _) = Pubkey::find_program_address(
        &[bonding_curve.as_ref(), Pubkey::from_str(TOKEN_PROGRAM).unwrap().as_ref(), mint.as_ref()],
        &Pubkey::from_str(ASSOCIATED_TOKEN_PROGRAM).unwrap(),
    );
    let (associated_user_account, _) = Pubkey::find_program_address(
        &[signer.as_ref(), Pubkey::from_str(TOKEN_PROGRAM).unwrap().as_ref(), mint.as_ref()],
        &Pubkey::from_str(ASSOCIATED_TOKEN_PROGRAM).unwrap(),
    );

    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(&[0x66, 0x06, 0x3d, 0x12, 0x01, 0xda, 0xeb, 0xea]); // Discriminator
    
    let amount = (req.amount_sol * 1_000_000_000.0) as u64; 
    data.extend_from_slice(&amount.to_le_bytes());
    data.extend_from_slice(&amount.to_le_bytes()); // Max SOL cost

    let ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new_readonly(Pubkey::from_str(GLOBAL).unwrap(), false),
            AccountMeta::new(Pubkey::from_str(FEE_RECIPIENT).unwrap(), false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(bonding_curve, false),
            AccountMeta::new(associated_bonding_curve, false),
            AccountMeta::new(associated_user_account, false),
            AccountMeta::new(signer, true),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(Pubkey::from_str(TOKEN_PROGRAM).unwrap(), false),
            AccountMeta::new_readonly(rent::ID, false),
            AccountMeta::new_readonly(Pubkey::from_str(EVENT_AUTHORITY).unwrap(), false),
            AccountMeta::new_readonly(program_id, false),
        ],
        data,
    };

    Ok(TradePlan {
        launchpad: Launchpad::PumpFun,
        instructions: vec![ix],
        signer_pubkey: signer,
        simulate: defaults.simulate_before_send,
    })
}

pub fn build_sell_plan(req: SellRequest, defaults: &crate::router::TradeDefaults) -> EngineResult<TradePlan> {
    let program_id = Pubkey::from_str(PUMP_FUN_PROGRAM_ID).unwrap();
    let mint = Pubkey::from_str(&req.token_mint).map_err(|_| EngineError::BadRequest("invalid mint".into()))?;
    let signer = Pubkey::from_str(&defaults.default_signer).unwrap_or_else(|_| Pubkey::new_unique());

    // Real PDA Derivations for Pump.fun
    let (bonding_curve, _) = Pubkey::find_program_address(&[b"bonding-curve", mint.as_ref()], &program_id);
    let (associated_bonding_curve, _) = Pubkey::find_program_address(
        &[bonding_curve.as_ref(), Pubkey::from_str(TOKEN_PROGRAM).unwrap().as_ref(), mint.as_ref()],
        &Pubkey::from_str(ASSOCIATED_TOKEN_PROGRAM).unwrap(),
    );
    let (associated_user_account, _) = Pubkey::find_program_address(
        &[signer.as_ref(), Pubkey::from_str(TOKEN_PROGRAM).unwrap().as_ref(), mint.as_ref()],
        &Pubkey::from_str(ASSOCIATED_TOKEN_PROGRAM).unwrap(),
    );

    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(&[0x33, 0xe6, 0x85, 0xa4, 0x01, 0x7f, 0x83, 0xad]); // Discriminator
    
    data.extend_from_slice(&req.amount_tokens.to_le_bytes());
    data.extend_from_slice(&0u64.to_le_bytes()); // Min SOL output

    let ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new_readonly(Pubkey::from_str(GLOBAL).unwrap(), false),
            AccountMeta::new(Pubkey::from_str(FEE_RECIPIENT).unwrap(), false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(bonding_curve, false),
            AccountMeta::new(associated_bonding_curve, false),
            AccountMeta::new(associated_user_account, false),
            AccountMeta::new(signer, true),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(Pubkey::from_str(TOKEN_PROGRAM).unwrap(), false),
            AccountMeta::new_readonly(Pubkey::from_str(EVENT_AUTHORITY).unwrap(), false),
            AccountMeta::new_readonly(program_id, false),
        ],
        data,
    };

    Ok(TradePlan {
        launchpad: Launchpad::PumpFun,
        instructions: vec![ix],
        signer_pubkey: signer,
        simulate: defaults.simulate_before_send,
    })
}
