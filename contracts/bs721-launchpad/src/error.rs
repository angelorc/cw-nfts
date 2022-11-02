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

    #[error("Collection is sold out!")]
    CollectionSoldOut {},
}
