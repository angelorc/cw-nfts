use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::Stage;

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

pub const CONFIG: Item<Config> = Item::new("config");
pub const SEED: Item<[u8; 32]> = Item::new("seed");

pub const STAGES: Map<u8, Stage> = Map::new("stages");
pub const STAGES_REMAINING: Item<Uint128> = Item::new("stages_remaining");
pub const STAGE_COUNTER: Item<u8> = Item::new("stage_counter");
pub const STAGE_REMAINING: Map<u8, Uint128> = Map::new("stage_remaining");

pub const NFT_ADDRESS: Item<Addr> = Item::new("nft_address");
pub const NFT_POSITIONS: Map<u128, u128> = Map::new("nft_positions");