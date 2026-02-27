
use axum::{routing::{get, post}, Json, Router, http::StatusCode};
use common::types::{BuyRequest, SellRequest, TradeResponse};
use common::error::EngineError;
use executor::engine::ExecutionEngine;
use strategies::router::{self, TradeDefaults};
use serde::Deserialize;
use std::{net::SocketAddr, sync::Arc};
use solana_sdk::{pubkey::Pubkey, signature::{read_keypair_file, Signer}};
use tracing_subscriber::EnvFilter;

/// Offset in the bonding-curve account data where the 32-byte creator pubkey lives.
/// Layout (after 8-byte Anchor discriminator):
///   5 × u64 fields (40 bytes) + 1 bool (1 byte) = 49 bytes before creator.
const BONDING_CURVE_CREATOR_OFFSET: usize = 49;

/// Derive the bonding-curve PDA for a given mint.
fn bonding_curve_pda(mint: &Pubkey) -> Pubkey {
    let program_id: Pubkey = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P".parse().unwrap();
    Pubkey::find_program_address(&[b"bonding-curve", mint.as_ref()], &program_id).0
}

/// Fetch the creator pubkey from the bonding curve account data.
fn fetch_creator_from_bonding_curve(
    rpc: &solana_client::rpc_client::RpcClient,
    mint: &Pubkey,
) -> Option<String> {
    let bc_pda = bonding_curve_pda(mint);
    let account = rpc.get_account(&bc_pda).ok()?;
    let data = account.data;
    // Need at least offset + 32 bytes
    if data.len() < BONDING_CURVE_CREATOR_OFFSET + 32 {
        return None;
    }
    let creator_bytes: [u8; 32] = data[BONDING_CURVE_CREATOR_OFFSET..BONDING_CURVE_CREATOR_OFFSET + 32]
        .try_into()
        .ok()?;
    Some(Pubkey::from(creator_bytes).to_string())
}

#[derive(Clone)]
struct AppState {
    engine: Arc<ExecutionEngine>,
    defaults: TradeDefaults,
}

#[derive(Deserialize)]
struct Config {
    rpc: RpcConfig,
    wallet: WalletConfig,
    trading: TradingConfig,
    service: ServiceConfig,
}

#[derive(Deserialize)]
struct RpcConfig { 
    http_url: String,
    jito_url: Option<String>,
}
#[derive(Deserialize)]
struct WalletConfig { keypair_path: String }
#[derive(Deserialize)]
struct TradingConfig { max_slippage_bps: u16, simulate_before_send: bool }
#[derive(Deserialize)]
struct ServiceConfig { bind_addr: String, log_level: String }

fn load_config() -> Result<Config, String> {
    let path = "config/config.toml";
    let raw = std::fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))?;
    toml::from_str(&raw).map_err(|e| format!("parse {path}: {e}"))
}

#[tokio::main]
async fn main() {
    let cfg = load_config().unwrap_or_else(|e| panic!("config error: {e}. Run: cp config/config.example.toml config/config.toml"));

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(cfg.service.log_level.clone()))
        .init();

    let kp_path = shellexpand::tilde(&cfg.wallet.keypair_path).to_string();
    let keypair = read_keypair_file(&kp_path)
        .unwrap_or_else(|_| solana_sdk::signature::Keypair::from_base58_string(&cfg.wallet.keypair_path));

    let state = AppState {
        engine: Arc::new(ExecutionEngine::new(
            cfg.rpc.http_url.clone(), 
            cfg.wallet.keypair_path.clone(),
            cfg.rpc.jito_url.clone()
        )),
        defaults: TradeDefaults {
            max_slippage_bps: cfg.trading.max_slippage_bps,
            simulate_before_send: cfg.trading.simulate_before_send,
            default_signer: keypair.pubkey().to_string(),
        },
    };

    let app = Router::new()
        .route("/v1/health", get(|| async { "ok" }))
        .route("/v1/trade/buy", post(buy))
        .route("/v1/trade/sell", post(sell))
        .route("/v1/solana/memo_fast", post(memo_fast))
        .with_state(state);

    let addr: SocketAddr = cfg.service.bind_addr.parse().expect("invalid bind_addr");
    let listener = tokio::net::TcpListener::bind(&addr).await.expect("failed to bind addr");
    println!("API listening on http://{addr}");
    axum::serve(listener, app).await.unwrap();
}

async fn buy(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<BuyRequest>,
) -> Result<Json<TradeResponse>, (StatusCode, Json<TradeResponse>)> {
    let mut req = req;
    // PRE-FLIGHT CHECK: Prevent "Incorrect Program ID" on Solscan
    // Users often accidentally paste the Bonding Curve address instead of the Token Mint address.
    // We verify the address belongs to the SPL Token Program.
    if let Ok(mint_pubkey) = std::str::FromStr::from_str(&req.token_mint) {
        match state.engine.rpc_client().get_account(&mint_pubkey) {
            Ok(account) => {
                let token_program = std::str::FromStr::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
                let token2022_program = std::str::FromStr::from_str("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb").unwrap();
                if account.owner != token_program && account.owner != token2022_program {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(TradeResponse {
                            success: false,
                            signature: None,
                            error: Some("CRITICAL ERROR: The address you provided is NOT a valid Token Mint! You likely copied the Bonding Curve address or Dev Wallet by mistake. Please copy the actual Token Contract Address.".to_string())
                        })
                    ));
                }
                
                let mut modified_req = req.clone();
                // Auto-detect correct token program (SPL vs Token-2022)
                modified_req.token_program = Some(account.owner.to_string());
                // Auto-fetch the creator from the bonding curve so creator_vault PDA is correct
                if modified_req.creator.is_none() {
                    if let Ok(mint_pk) = req.token_mint.parse::<Pubkey>() {
                        modified_req.creator = fetch_creator_from_bonding_curve(
                            state.engine.rpc_client(), &mint_pk
                        );
                    }
                }
                req = modified_req;
            }
            Err(_) => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(TradeResponse {
                        success: false,
                        signature: None,
                        error: Some("CRITICAL ERROR: Token Mint not found on-chain. Please ensure you are providing a valid pump.fun token mint address.".to_string())
                    })
                ));
            }
        }
    } else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(TradeResponse {
                success: false,
                signature: None,
                error: Some("CRITICAL ERROR: Invalid token mint format.".to_string())
            })
        ));
    }


    let plan = router::build_buy_plan(req, &state.defaults).map_err(map_err)?;
    let sig = state.engine.execute(plan).await.map_err(map_err)?;
    Ok(Json(TradeResponse { success: true, signature: Some(sig), error: None }))
}

async fn sell(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<SellRequest>,
) -> Result<Json<TradeResponse>, (StatusCode, Json<TradeResponse>)> {
    let mut req = req;
    if let Ok(mint_pubkey) = std::str::FromStr::from_str(&req.token_mint) {
        match state.engine.rpc_client().get_account(&mint_pubkey) {
            Ok(account) => {
                let token_program = std::str::FromStr::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
                let token2022_program = std::str::FromStr::from_str("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb").unwrap();
                if account.owner != token_program && account.owner != token2022_program {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(TradeResponse {
                            success: false,
                            signature: None,
                            error: Some("CRITICAL ERROR: The address you provided is NOT a valid Token Mint! You likely copied the Bonding Curve address or Dev Wallet by mistake. Please copy the actual Token Contract Address.".to_string())
                        })
                    ));
                }
                
                let mut modified_req = req.clone();
                // Auto-detect correct token program (SPL vs Token-2022)
                modified_req.token_program = Some(account.owner.to_string());
                // Auto-fetch the creator from the bonding curve so creator_vault PDA is correct
                if modified_req.creator.is_none() {
                    if let Ok(mint_pk) = req.token_mint.parse::<Pubkey>() {
                        modified_req.creator = fetch_creator_from_bonding_curve(
                            state.engine.rpc_client(), &mint_pk
                        );
                    }
                }
                req = modified_req;
            }
            Err(_) => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(TradeResponse {
                        success: false,
                        signature: None,
                        error: Some("CRITICAL ERROR: Token Mint not found on-chain. Please ensure you are providing a valid pump.fun token mint address.".to_string())
                    })
                ));
            }
        }
    } else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(TradeResponse {
                success: false,
                signature: None,
                error: Some("CRITICAL ERROR: Invalid token mint format.".to_string())
            })
        ));
    }

    let plan = router::build_sell_plan(req, &state.defaults).map_err(map_err)?;
    let sig = state.engine.execute(plan).await.map_err(map_err)?;
    Ok(Json(TradeResponse { success: true, signature: Some(sig), error: None }))
}

#[derive(Deserialize)]
struct MemoRequest { memo: String }

async fn memo_fast(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<MemoRequest>,
) -> Result<Json<TradeResponse>, (StatusCode, Json<TradeResponse>)> {
    let ix = solana_sdk::instruction::Instruction {
        program_id: std::str::FromStr::from_str("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr").unwrap(),
        accounts: vec![],
        data: req.memo.into_bytes(),
    };
    
    let plan = common::types::TradePlan {
        launchpad: common::types::Launchpad::PumpFun, // Placeholder
        instructions: vec![ix],
        signer_pubkey: std::str::FromStr::from_str(&state.defaults.default_signer).unwrap_or_else(|_| solana_sdk::pubkey::Pubkey::new_unique()),
        simulate: false,
    };
    
    let sig = state.engine.execute(plan).await.map_err(map_err)?;
    Ok(Json(TradeResponse { success: true, signature: Some(sig), error: None }))
}

fn map_err(e: EngineError) -> (StatusCode, Json<TradeResponse>) {
    let (code, msg) = match &e {
        EngineError::BadRequest(_) => (StatusCode::BAD_REQUEST, e.to_string()),
        EngineError::NotImplemented(_) => (StatusCode::NOT_IMPLEMENTED, e.to_string()),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    (code, Json(TradeResponse { success: false, signature: None, error: Some(msg) }))
}
