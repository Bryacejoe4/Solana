
use axum::{routing::{get, post}, Json, Router, http::StatusCode};
use common::types::{BuyRequest, SellRequest, TradeResponse};
use common::error::EngineError;
use executor::engine::ExecutionEngine;
use strategies::router::{self, TradeDefaults};
use serde::Deserialize;
use std::{net::SocketAddr, sync::Arc};
use solana_sdk::signature::{read_keypair_file, Signer};
use tracing_subscriber::EnvFilter;

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
    let keypair = read_keypair_file(&kp_path).unwrap_or_else(|e| panic!("failed to load keypair at {kp_path}: {e}"));

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
    let plan = router::build_buy_plan(req, &state.defaults).map_err(map_err)?;
    let sig = state.engine.execute(plan).await.map_err(map_err)?;
    Ok(Json(TradeResponse { success: true, signature: Some(sig), error: None }))
}

async fn sell(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<SellRequest>,
) -> Result<Json<TradeResponse>, (StatusCode, Json<TradeResponse>)> {
    let plan = router::build_sell_plan(req, &state.defaults).map_err(map_err)?;
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
