use cosmwasm_std::{Addr, Coin};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub admin: Addr,
    pub name: String,
    pub symbol: String,
    pub collection_uri: Option<String>,
    pub base_token_uri: String,
    pub seller_fee_bps: u16,
    pub payment_address: Option<Addr>,
    pub price: Coin,
}

// TODO: add start and end time
// TODO: add update price

pub const CONFIG: Item<Config> = Item::new("config");
pub const SEED: Item<[u8; 32]> = Item::new("seed");
pub const NFT_ADDRESS: Item<Option<Addr>> = Item::new("nft_address");
pub const NFT_POSITIONS: Map<u32, u32> = Map::new("nft_positions");
pub const NFT_REMAINING: Item<u32> = Item::new("nft_remaining");