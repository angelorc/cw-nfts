use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("token_id already claimed")]
    Claimed {},

    #[error("Cannot set approval that is already expired")]
    Expired {},

    #[error("Approval not found for: {spender}")]
    ApprovalNotFound { spender: String },

    #[error("Creators list must be at least 1")]
    CreatorsTooShort {},
    
    #[error("Creators list too long")]
    CreatorsTooLong {},

    #[error("Creators must be at least one if set")]
    CreatorsMustBeAtleastOne {},

    #[error("Basis points in seller fee cannot exceed 10000")]
    SellerFeeBasisPointsTooHigh {},

    #[error("Creator shares must sum to 100")]
    CreatorShareTotalMustBe100 {},

    #[error("No duplicate creator addresses allowed")]
    DuplicateCreatorAddress {},
}
