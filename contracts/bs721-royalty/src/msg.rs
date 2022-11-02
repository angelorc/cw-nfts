use cosmwasm_schema::cw_serde;

use crate::state::Config;

#[cw_serde]
pub struct InstantiateMsg {
    pub cw4_address: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    PayMembers {}
}

#[cw_serde]
pub enum QueryMsg {
    Config {},
    Member { address: String },
    AllMembers {
        start_after: Option<String>,
        limit: Option<u32>,
    }
}

#[cw_serde]
pub struct ConfigResponse {
    pub config: Config,
}