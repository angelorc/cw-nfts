use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin};
use cw_utils::{Scheduled, Expiration};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cw_serde]
pub struct Stage {
    pub merkle_root: Option<String>,
    pub start: Option<Scheduled>,
    pub expiration: Option<Expiration>,
    pub price: Option<Coin>,
    pub total_amount: u32,
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
    pub stages: Vec<Stage>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Mint {
        stage: u8,
        proofs: Option<Vec<String>>,
    },
    // AddStage,
    // PauseStage,
    // ResumeStage,
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
    pub stages: Vec<StageResponse>
}

#[cw_serde]
pub struct StageResponse {
    pub id: u8,
    pub merkle_root: Option<String>,
    pub start: Option<Scheduled>,
    pub expiration: Option<Expiration>,
    pub price: Option<Coin>,
    pub total_amount: u32,
}