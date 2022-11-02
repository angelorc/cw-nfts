use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: String,
    pub name: String,
    pub symbol: String,
    pub collection_uri: Option<String>,
    pub base_token_uri: String,
    pub seller_fee: u16,
    pub payment_address: Option<String>,
    pub price: Coin,
    pub nft_limit: u32,
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
    pub price: Coin,
    pub nft_address: Option<Addr>,
    pub nft_remaining: u32,
}