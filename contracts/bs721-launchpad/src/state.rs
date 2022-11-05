use std::ops::Add;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Timestamp, Coin};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::ContractError;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub admin: Addr,
    pub name: String,
    pub symbol: String,
    pub collection_uri: Option<String>,
    pub base_token_uri: String,
    pub seller_fee_bps: u16,
    pub payment_address: Option<Addr>,
}

#[cw_serde]
pub struct Stage {
    pub label: String,
    pub start_date: Timestamp,
    pub end_date: Timestamp,
    pub max_supply: u32,
    pub supply: u32,
    pub price: Option<Coin>
}

#[cw_serde]
pub struct Stages {
    pub free_mint: Option<Stage>,
    pub pre_sale: Option<Stage>,
    pub public_sale: Stage
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const SEED: Item<[u8; 32]> = Item::new("seed");
pub const STAGES: Item<Stages> = Item::new("stages");
pub const NFT_ADDRESS: Item<Addr> = Item::new("nft_address");
pub const NFT_POSITIONS: Map<u32, u32> = Map::new("nft_positions");

impl Stages {
    pub fn max_supply(&self) -> u32 {
        let mut max_supply = self.public_sale.max_supply;

        let free_mint = self.free_mint.clone();
        match free_mint {
            Some(param) => max_supply = max_supply.add(param.max_supply),
            None => {},
        }

        let pre_sale = self.pre_sale.clone();
        match pre_sale {
            Some(param) => max_supply = max_supply.add(param.max_supply),
            None => {},
        }

        max_supply
    }

    pub fn remaining_supply(&self) -> u32 {
        let mut supply = self.public_sale.supply;
        
        let free_mint = self.free_mint.clone();
        match free_mint {
            Some(param) => supply = supply.add(param.supply),
            None => {},
        }

        let pre_sale = self.pre_sale.clone();
        match pre_sale {
            Some(param) => supply = supply.add(param.supply),
            None => {},
        }

        self.max_supply() - supply
    }

    pub fn current_stage(&self, block_time: Timestamp) -> Result<Stage, ContractError> {
        let free_mint = self.free_mint.clone();
        match &free_mint {
            Some(stage) => {
                if block_time > stage.start_date && block_time < stage.end_date  {
                    return Ok(free_mint.unwrap())
                }
            }
            None => {},
        }

        let pre_sale = self.pre_sale.clone();
        match &pre_sale {
            Some(stage) => {
                if block_time > stage.start_date && block_time < stage.end_date  {
                    return Ok(pre_sale.unwrap())
                }
            }
            None => {},
        }

        let public_sale = self.public_sale.clone();
        if block_time > public_sale.start_date && block_time < public_sale.end_date {
            return Ok(public_sale.clone())
        };

        Err(ContractError::NoActiveStages {  })
    }
}

impl Stage {
    pub fn remaining(&self) -> u32 {
        self.max_supply - self.supply
    }

    pub fn validate(&self, block_time: Timestamp) -> Result<(), ContractError> {
        if self.start_date.clone() < block_time {
            return Err(ContractError::StageStartDate {  })
        }

        if self.end_date.clone() <= self.start_date.clone() {
            return Err(ContractError::StageEndDate {  })
        }

        /*if self.max_supply.clone() == 0 {
            return Err(ContractError::StageInvalidMaxSupply {  })
        }*/

        match self.price.clone() {
            Some(price) => {
                if price.amount.is_zero() {
                    return Err(ContractError::StageInvalidPrice {  })    
                }
            },
            _ => {},
        }

        Ok(())
    }
}