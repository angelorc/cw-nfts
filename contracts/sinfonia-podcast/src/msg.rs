use cosmwasm_schema::cw_serde;
use cosmwasm_std::CustomMsg;

#[cw_serde]
pub struct ItunesDataMsg {
    pub author: Option<String>,
    pub category: Option<Vec<String>>,
    pub pod_type: Option<String>,
    pub image: Option<String>,
}

#[cw_serde]
pub struct InstantiateMsg {  
    /// Symbol of the NFT contract
    pub symbol: String,

    /// The minter is the only one who can create new NFTs.
    /// This is designed for a base NFT that is controlled by an external program
    /// or contract. You will likely replace this with custom logic in custom NFTs
    pub minter: String,

    pub title: String,
    pub description: String,
    pub link: Option<String>,
    pub lang: Option<String>,
    pub itunes: ItunesDataMsg,
}

#[cw_serde]
pub enum SiPodcastQueryMsg {
    PodcastInfo {  }
}

impl CustomMsg for SiPodcastQueryMsg {}