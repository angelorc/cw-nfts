use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Podcast {
    pub title: String,
    pub description: String,
    pub link: Option<String>,
}

pub const PODCAST: Item<Podcast> = Item::new("Podcast");