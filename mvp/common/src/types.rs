
use serde::{Deserialize, Serialize};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

#[derive(Debug, Deserialize)]
pub struct BuyRequest {
    pub launchpad: String,
    pub token_mint: String,
    pub amount_sol: f64,
    pub max_slippage_bps: Option<u16>,
}

#[derive(Debug, Deserialize)]
pub struct SellRequest {
    pub launchpad: String,
    pub token_mint: String,
    pub amount_tokens: u64,
    pub max_slippage_bps: Option<u16>,
}

#[derive(Debug, Serialize)]
pub struct TradeResponse {
    pub success: bool,
    pub signature: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum Launchpad {
    PumpFun,
    PumpFunAmm,
    Bags,
    Unknown,
}

impl Launchpad {
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pumpfun" => Self::PumpFun,
            "pumpfun_amm" | "pumpfunamm" => Self::PumpFunAmm,
            "bags" => Self::Bags,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug)]
pub struct TradePlan {
    pub launchpad: Launchpad,
    pub instructions: Vec<Instruction>,
    pub signer_pubkey: Pubkey,
    pub simulate: bool,
}
