use cosmwasm_std::{Coin, Timestamp, Addr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub owner: String,
    pub payment_addr: Option<Addr>,
    pub mint_cost: Coin,
    pub name: String,
    pub symbol: String,
    pub uri: Option<String>,
    pub base_token_uri: String,
    pub nft_limit: u32,
    pub limit_per_address: u64,
    pub start_time: Timestamp,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CreateMsg<T> {
    pub init_msg: T,
    pub collection_params: CollectionParams,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CollectionParams {
    pub code_id: u64,
    pub name: String,
    pub symbol: String,
    pub uri: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {

}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {

}