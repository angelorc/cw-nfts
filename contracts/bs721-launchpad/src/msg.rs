use std::ops::Add;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Timestamp};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::{Stages, Stage};

#[cw_serde]
pub struct StageMsgWithPrice {
    pub start_date: Timestamp,
    pub end_date: Timestamp,
    pub max_supply: u32,
    pub price: Coin,
}

#[cw_serde]
pub struct StageMsgNoPrice {
    pub start_date: Timestamp,
    pub end_date: Timestamp,
    pub max_supply: u32,
}

#[cw_serde]
pub struct StagesMsg {
    pub free_mint: Option<StageMsgNoPrice>,
    pub pre_sale: Option<StageMsgWithPrice>,
    pub public_sale: StageMsgWithPrice
}

impl StagesMsg {
    pub fn total_supply(&self) -> u32 {
        let mut total_supply = 0;

        total_supply = total_supply.add(&self.public_sale.max_supply);

        match self.pre_sale.clone() {
            Some(param) => {
                total_supply = total_supply.add(param.max_supply);
            },
            _ => {}
        }

        match self.free_mint.clone() {
            Some(param) => {
                total_supply = total_supply.add(param.max_supply);
            },
            _ => {}
        };

        total_supply
    }

    pub fn to_stages(&self) -> Stages {
        let mut stages = Stages {
            free_mint: None,
            pre_sale: None,
            public_sale: Stage{
                label: "public_sale".to_string(),
                start_date: self.public_sale.clone().start_date,
                end_date: self.public_sale.clone().end_date,
                supply: 0,
                max_supply: self.public_sale.clone().max_supply,
                price: Some(self.public_sale.clone().price.clone()),
            }
        };

        let free_mint = self.free_mint.clone();

        match free_mint {
            Some(param) => {
                stages.free_mint = Some(Stage {
                    label: "free_mint".to_string(),
                    start_date: param.start_date,
                    end_date: param.end_date,
                    supply: 0,
                    max_supply: param.max_supply,
                    price: None,
                });
            },
            _ => {},
        }

        let pre_sale = self.pre_sale.clone();

        match pre_sale {
            Some(param) => {
                stages.pre_sale = Some(Stage {
                    label: "pre_sale".to_string(),
                    start_date: param.start_date,
                    end_date: param.end_date,
                    supply: 0,
                    max_supply: param.max_supply,
                    price: Some(self.public_sale.clone().price),
                });
            },
            _ => {},
        }

        stages
    }
}

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: String,
    pub name: String,
    pub symbol: String,
    pub collection_uri: Option<String>,
    pub base_token_uri: String,
    pub seller_fee: u16,
    pub payment_address: Option<String>,
    pub stages: StagesMsg,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Mint {},
    // TODO:
    // Add a finalize msg that clean the nft_positions and the nft_remaining storage
    // Should we add an incentive? May the creator should deposit an amount of tokens
    // then he can withdraw the deposit by sending the Finalize {} msg.
}

#[cw_serde]
pub enum QueryMsg {
    Config {},
}

#[cw_serde]
pub struct ConfigResponse {
    pub admin: Addr,
    pub name: String,
    pub symbol: String,
    pub collection_uri: Option<String>,
    pub base_token_uri: String,
    pub seller_fee_bps: u16,
    pub payment_address: Option<Addr>,
    pub nft_address: Addr,
    pub stages: Stages
}