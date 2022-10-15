use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Unknown reply id")]
    UnknownReplyId {},

    #[error("Instantiate contract error")]
    InstantiateContractError {},

    #[error("InvalidUnitPrice")]
    InvalidUnitPrice {},

    #[error("InvalidNftLimit")]
    InvalidNftLimit {},

    #[error("BS721AlreadyLinked")]
    BS721AlreadyLinked {},
}
