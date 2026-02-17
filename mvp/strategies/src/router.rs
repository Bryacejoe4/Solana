
use common::error::{EngineError, EngineResult};
use common::types::{BuyRequest, SellRequest, TradePlan, Launchpad};
use crate::pumpfun;

#[derive(Clone)]
pub struct TradeDefaults {
    pub max_slippage_bps: u16,
    pub simulate_before_send: bool,
}

pub fn build_buy_plan(req: BuyRequest, defaults: &TradeDefaults) -> EngineResult<TradePlan> {
    match Launchpad::parse(&req.launchpad) {
        Launchpad::PumpFun => pumpfun::build_buy_plan(req, defaults),
        Launchpad::PumpFunAmm => Err(EngineError::NotImplemented("Pump.fun AMM not implemented yet".into())),
        Launchpad::Bags => Err(EngineError::NotImplemented("Bags not implemented yet".into())),
        Launchpad::Unknown => Err(EngineError::BadRequest("unknown launchpad".into())),
    }
}

pub fn build_sell_plan(_req: SellRequest, _defaults: &TradeDefaults) -> EngineResult<TradePlan> {
    Err(EngineError::NotImplemented("sell not implemented yet".into()))
}
