
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("config error: {0}")]
    Config(String),
    #[error("rpc error: {0}")]
    Rpc(String),
    #[error("simulation failed: {0}")]
    Simulation(String),
    #[error("invalid request: {0}")]
    BadRequest(String),
    #[error("not implemented: {0}")]
    NotImplemented(String),
}

pub type EngineResult<T> = Result<T, EngineError>;
