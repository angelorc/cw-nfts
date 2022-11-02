use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},
    
    #[error("Contract has no coins")]
    NoCoins {},

    #[error("Invalid cw4 address '{addr}'")]
    InvalidCw4 { addr: String },

    #[error("Invalid cw4 total weight '{weight}'")]
    InvalidCw4TotalWeight { weight: u64 },
}
