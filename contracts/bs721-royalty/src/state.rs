use cw4::Cw4Contract;
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
    pub cw4_address: Cw4Contract,
}

pub const CONFIG: Item<Config> = Item::new("config");