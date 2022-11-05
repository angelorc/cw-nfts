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

    #[error("InvalidUnitPrice")]
    InvalidUnitPrice {},

    #[error("InvalidNftLimit")]
    InvalidNftLimit {},

    #[error("BS721AlreadyLinked")]
    BS721AlreadyLinked {},

    #[error("MaxLimitAddressReached")]
    MaxLimitAddressReached {},

    #[error("IncorrectPaymentAmount {0} != {1}")]
    IncorrectPaymentAmount(Coin, Coin),

    #[error("Basis points in seller fee cannot exceed 10000")]
    SellerFeeBasisPointsTooHigh {},

    #[error("Stage is sold out!")]
    StageSoldOut {},

    #[error("No active stages!")]
    NoActiveStages {},

    #[error("Start date must be greater then current time")]
    StageStartDate {},

    #[error("End date must be greater then start time")]
    StageEndDate {},

    #[error("Max supply must be greater then zero")]
    StageInvalidMaxSupply {},

    #[error("Invalid stage price, must be greater then zero")]
    StageInvalidPrice {},

    #[error("Invalid date")]
    InvalidDate {},
}
