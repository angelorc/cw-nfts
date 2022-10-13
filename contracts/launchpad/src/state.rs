use cosmwasm_std::{Addr, Timestamp, Coin};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
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

pub const CONFIG: Item<Config> = Item::new("config");
pub const NFT_ADDRESS: Item<Addr> = Item::new("nft_address");
pub const MINTABLE_TOKEN_POSITIONS: Map<u32, u32> = Map::new("mt");
pub const MINTABLE_NUM_TOKENS: Item<u32> = Item::new("mintable_num_tokens");
pub const MINTER_ADDRS: Map<&Addr, u32> = Map::new("ma");