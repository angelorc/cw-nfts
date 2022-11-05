use cosmwasm_std::Timestamp;

use crate::{msg::StagesMsg, error::ContractError};

const MAX_SELLER_FEE: u16 = 10000; // mean 100.00%

pub fn validate_stages(stages_msg: StagesMsg, block_time: Timestamp) -> Result<bool, ContractError> {
    let stages = &stages_msg.to_stages();
    
    let public_sale = stages.public_sale.clone();
    public_sale.validate(block_time)?;

    let free_mint = stages.clone().free_mint;
    let pre_sale = stages.clone().pre_sale;

    if stages.clone().max_supply() == 0  {
        return Err(ContractError::StageInvalidMaxSupply {})
    }

    match free_mint {
        Some(free_mint_stage) => {
            free_mint_stage.clone().validate(block_time)?;

            match pre_sale.clone() {
                Some(pre_sale_stage) => {
                    if pre_sale_stage.clone().start_date < free_mint_stage.clone().end_date {
                        return Err(ContractError::InvalidDate {  })
                    }
                }
                None => {},
            }
        },
        None => {},
    }

    match pre_sale.clone() {
        Some(pre_sale_stage) => {
            pre_sale_stage.validate(block_time)?;

            if public_sale.clone().start_date < pre_sale_stage.clone().end_date {
                return Err(ContractError::InvalidDate {  })
            }
        },
        None => {},
    }

    Ok(true)
}

pub fn validate_seller_fee(seller_fee: u16) -> Result<bool, ContractError> {
    if seller_fee > MAX_SELLER_FEE {
        return Err(ContractError::SellerFeeBasisPointsTooHigh { })
    }

    Ok(true)
}