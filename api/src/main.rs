
use axum::{routing::{get, post}, Json, Router, http::StatusCode};
use common::types::{BuyRequest, SellRequest, TradeResponse};
use common::error::EngineError;
use executor::engine::ExecutionEngine;
use strategies::router::{self, TradeDefaults};
use mev::{MevProvider, jito::JitoClient, nextblock::NextBlockClient, zeroblock::ZeroBlockClient};
use mev::router::{MevRouter, MevStrategy};
use serde::Deserialize;
use std::{net::SocketAddr, sync::Arc};
use solana_sdk::{pubkey::Pubkey, signature::{read_keypair_file, Signer}};
use solana_client::nonblocking::rpc_client::RpcClient as AsyncRpcClient;
use tracing_subscriber::EnvFilter;
use tracing::info;

/// Offset in the bonding-curve account data where the 32-byte creator pubkey lives.
const BONDING_CURVE_CREATOR_OFFSET: usize = 49;

fn bonding_curve_pda(mint: &Pubkey) -> Pubkey {
    let program_id: Pubkey = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P".parse().unwrap();
    Pubkey::find_program_address(&[b"bonding-curve", mint.as_ref()], &program_id).0
}

async fn fetch_creator_from_bonding_curve(
    rpc: &AsyncRpcClient,
    mint: &Pubkey,
) -> Option<String> {
    let bc_pda = bonding_curve_pda(mint);
    let account = rpc.get_account(&bc_pda).await.ok()?;
    let data = account.data;
    if data.len() < BONDING_CURVE_CREATOR_OFFSET + 32 {
        return None;
    }
    let creator_bytes: [u8; 32] = data[BONDING_CURVE_CREATOR_OFFSET..BONDING_CURVE_CREATOR_OFFSET + 32]
        .try_into()
        .ok()?;
    Some(Pubkey::from(creator_bytes).to_string())
}

async fn validate_mint(rpc: &AsyncRpcClient, mint_str: &str) -> Result<(Pubkey, String), String> {
    let mint_pubkey: Pubkey = mint_str.parse()
        .map_err(|_| "Invalid token mint format".to_string())?;

    let account = rpc.get_account(&mint_pubkey).await
        .map_err(|_| "Token Mint not found on-chain.".to_string())?;

    let token_program: Pubkey = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".parse().unwrap();
    let token2022_program: Pubkey = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb".parse().unwrap();

    if account.owner != token_program && account.owner != token2022_program {
        return Err("The address is NOT a valid Token Mint!".to_string());
    }

    Ok((mint_pubkey, account.owner.to_string()))
}

#[derive(Clone)]
struct AppState {
    engine: Arc<ExecutionEngine>,
    defaults: TradeDefaults,
    rpc: Arc<AsyncRpcClient>,
}

// ── Config ───────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct Config {
    rpc: RpcConfig,
    wallet: WalletConfig,
    trading: TradingConfig,
    service: ServiceConfig,
    #[serde(default)]
    mev: MevConfig,
}

#[derive(Deserialize)]
struct RpcConfig { 
    http_url: String,
    #[serde(default)]
    jito_url: Option<String>,
}

#[derive(Deserialize)]
struct WalletConfig { keypair_path: String }

#[derive(Deserialize)]
struct TradingConfig { max_slippage_bps: u16, simulate_before_send: bool }

#[derive(Deserialize)]
struct ServiceConfig { bind_addr: String, log_level: String }

#[derive(Deserialize, Default)]
struct MevConfig {
    /// Which providers to enable: "jito", "nextblock", "zeroblock"
    #[serde(default)]
    providers: Vec<String>,
    /// Routing strategy: "parallel" or "round_robin"
    #[serde(default = "default_strategy")]
    strategy: String,
    /// NextBlock endpoint URL
    #[serde(default)]
    nextblock_url: Option<String>,
    /// NextBlock API key
    #[serde(default)]
    nextblock_api_key: Option<String>,
    /// 0block endpoint URL
    #[serde(default)]
    zeroblock_url: Option<String>,
    /// 0block API key
    #[serde(default)]
    zeroblock_api_key: Option<String>,
}

fn default_strategy() -> String { "parallel".to_string() }

fn load_config() -> Result<Config, String> {
    let path = "config/config.toml";
    let raw = std::fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))?;
    toml::from_str(&raw).map_err(|e| format!("parse {path}: {e}"))
}

/// Build MEV providers from config.
fn build_mev_provider(cfg: &Config) -> Option<Arc<dyn MevProvider>> {
    let mut providers: Vec<Arc<dyn MevProvider>> = Vec::new();

    // If the old-style jito_url is set and no [mev] section, use Jito directly
    if cfg.mev.providers.is_empty() {
        if let Some(ref jito_url) = cfg.rpc.jito_url {
            info!("Using Jito (legacy config) at {}", jito_url);
            return Some(Arc::new(JitoClient::new(jito_url)));
        }
        return None;
    }

    for name in &cfg.mev.providers {
        match name.to_lowercase().as_str() {
            "jito" => {
                if let Some(ref url) = cfg.rpc.jito_url {
                    info!("Adding MEV provider: Jito at {}", url);
                    providers.push(Arc::new(JitoClient::new(url)));
                } else {
                    tracing::warn!("Jito listed in [mev].providers but no jito_url configured");
                }
            }
            "nextblock" => {
                if let Some(ref url) = cfg.mev.nextblock_url {
                    info!("Adding MEV provider: NextBlock at {}", url);
                    providers.push(Arc::new(NextBlockClient::new(url, cfg.mev.nextblock_api_key.clone())));
                } else {
                    tracing::warn!("NextBlock listed in [mev].providers but no nextblock_url configured");
                }
            }
            "zeroblock" | "0block" => {
                if let Some(ref url) = cfg.mev.zeroblock_url {
                    info!("Adding MEV provider: 0block at {}", url);
                    providers.push(Arc::new(ZeroBlockClient::new(url, cfg.mev.zeroblock_api_key.clone())));
                } else {
                    tracing::warn!("0block listed in [mev].providers but no zeroblock_url configured");
                }
            }
            other => {
                tracing::warn!("Unknown MEV provider '{}', skipping", other);
            }
        }
    }

    if providers.is_empty() {
        return None;
    }

    if providers.len() == 1 {
        return Some(providers.into_iter().next().unwrap());
    }

    let strategy = match cfg.mev.strategy.to_lowercase().as_str() {
        "round_robin" | "roundrobin" => MevStrategy::RoundRobin,
        _ => MevStrategy::Parallel,
    };

    Some(Arc::new(MevRouter::new(providers, strategy)))
}

// ── Main ─────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let cfg = load_config().unwrap_or_else(|e| panic!("config error: {e}. Run: cp config/config.example.toml config/config.toml"));

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(cfg.service.log_level.clone()))
        .init();

    let kp_path = shellexpand::tilde(&cfg.wallet.keypair_path).to_string();
    let keypair = read_keypair_file(&kp_path)
        .unwrap_or_else(|_| solana_sdk::signature::Keypair::from_base58_string(&cfg.wallet.keypair_path));

    let rpc = Arc::new(AsyncRpcClient::new(cfg.rpc.http_url.clone()));
    let mev_provider = build_mev_provider(&cfg);

    let state = AppState {
        engine: Arc::new(ExecutionEngine::new(
            cfg.rpc.http_url.clone(), 
            cfg.wallet.keypair_path.clone(),
            mev_provider,
        )),
        defaults: TradeDefaults {
            max_slippage_bps: cfg.trading.max_slippage_bps,
            simulate_before_send: cfg.trading.simulate_before_send,
            default_signer: keypair.pubkey().to_string(),
        },
        rpc,
    };

    let app = Router::new()
        .route("/v1/health", get(|| async { "ok" }))
        .route("/v1/trade/buy", post(buy))
        .route("/v1/trade/sell", post(sell))
        .route("/v1/trade/cancel", post(cancel))
        .route("/v1/solana/memo_fast", post(memo_fast))
        .with_state(state);

    let addr: SocketAddr = cfg.service.bind_addr.parse().expect("invalid bind_addr");
    let listener = tokio::net::TcpListener::bind(&addr).await.expect("failed to bind addr");
    println!("API listening on http://{addr}");
    axum::serve(listener, app).await.unwrap();
}

// ── Handlers ─────────────────────────────────────────────────────────────────

async fn buy(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<BuyRequest>,
) -> Result<Json<TradeResponse>, (StatusCode, Json<TradeResponse>)> {
    let mut req = req;
    let (mint_pk, token_program) = validate_mint(&state.rpc, &req.token_mint).await
        .map_err(|msg| bad_request(&msg))?;
    req.token_program = Some(token_program);
    if req.creator.is_none() {
        req.creator = fetch_creator_from_bonding_curve(&state.rpc, &mint_pk).await;
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
    let (mint_pk, token_program) = validate_mint(&state.rpc, &req.token_mint).await
        .map_err(|msg| bad_request(&msg))?;
    req.token_program = Some(token_program);
    if req.creator.is_none() {
        req.creator = fetch_creator_from_bonding_curve(&state.rpc, &mint_pk).await;
    }
    let plan = router::build_sell_plan(req, &state.defaults).map_err(map_err)?;
    let sig = state.engine.execute(plan).await.map_err(map_err)?;
    Ok(Json(TradeResponse { success: true, signature: Some(sig), error: None }))
}

#[derive(Deserialize)]
struct CancelRequest { token_mint: String }

async fn cancel(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<CancelRequest>,
) -> Result<Json<TradeResponse>, (StatusCode, Json<TradeResponse>)> {
    let cancelled = state.engine.cancel_trade(&req.token_mint).await;
    if cancelled {
        Ok(Json(TradeResponse { success: true, signature: None, error: None }))
    } else {
        Err((StatusCode::NOT_FOUND, Json(TradeResponse {
            success: false, signature: None,
            error: Some("No active trade found for this token".to_string()),
        })))
    }
}

#[derive(Deserialize)]
struct MemoRequest { memo: String }

async fn memo_fast(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<MemoRequest>,
) -> Result<Json<TradeResponse>, (StatusCode, Json<TradeResponse>)> {
    let ix = solana_sdk::instruction::Instruction {
        program_id: "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr".parse().unwrap(),
        accounts: vec![],
        data: req.memo.into_bytes(),
    };
    let plan = common::types::TradePlan {
        launchpad: common::types::Launchpad::PumpFun,
        instructions: vec![ix],
        signer_pubkey: state.defaults.default_signer.parse().unwrap_or_else(|_| Pubkey::new_unique()),
        simulate: false,
    };
    let sig = state.engine.execute(plan).await.map_err(map_err)?;
    Ok(Json(TradeResponse { success: true, signature: Some(sig), error: None }))
}

// ── Error Mapping ────────────────────────────────────────────────────────────

fn bad_request(msg: &str) -> (StatusCode, Json<TradeResponse>) {
    (StatusCode::BAD_REQUEST, Json(TradeResponse {
        success: false, signature: None, error: Some(msg.to_string()),
    }))
}

fn map_err(e: EngineError) -> (StatusCode, Json<TradeResponse>) {
    let (code, msg) = match &e {
        EngineError::BadRequest(_) => (StatusCode::BAD_REQUEST, e.to_string()),
        EngineError::NotImplemented(_) => (StatusCode::NOT_IMPLEMENTED, e.to_string()),
        EngineError::Confirmation(_) => (StatusCode::UNPROCESSABLE_ENTITY, e.to_string()),
        EngineError::Timeout(_) => (StatusCode::GATEWAY_TIMEOUT, e.to_string()),
        EngineError::Cancelled(_) => (StatusCode::from_u16(499).unwrap_or(StatusCode::BAD_REQUEST), e.to_string()),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    (code, Json(TradeResponse { success: false, signature: None, error: Some(msg) }))
}
