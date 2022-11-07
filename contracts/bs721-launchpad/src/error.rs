use cosmwasm_std::{StdError, Coin};
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

    #[error("BS721AlreadyLinked")]
    BS721AlreadyLinked {},

    #[error("IncorrectPaymentAmount {0} != {1}")]
    IncorrectPaymentAmount(Coin, Coin),

    #[error("Basis points in seller fee cannot exceed 10000")]
    SellerFeeBasisPointsTooHigh {},

    #[error("Add at least one stage")]
    StageRequired {},

    #[error("Stage is sold out!")]
    StageSoldOut {},

    #[error("Stage not found")]
    StageNotFound {},

    #[error("Stage not begun")]
    StageNotBegun {},

    #[error("Start is expired")]
    StageExpired {},

    #[error("Stage invalid supply")]
    StageInvalidSupply {},

    #[error("Wrong length")]
    WrongLength {},

    #[error("Verification failed")]
    VerificationFailed {},
}
