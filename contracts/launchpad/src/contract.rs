use cosmwasm_std::{
    entry_point, to_binary, Addr, DepsMut, Env, MessageInfo, Reply, ReplyOn, Response, StdError,
    StdResult, SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use cw721_base::InstantiateMsg as Cw721InstantiateMsg;
use cw_utils::parse_reply_instantiate_data;

use crate::msg::InstantiateMsg;
use crate::state::{Config, CONFIG, MINTABLE_NFT_LIMIT, NFT_ADDRESS};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:bs721-launchpad";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANTIATE_BS721_CODE_ID: u64 = 11;
const INSTANTIATE_REPLY_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let cfg = Config {
        owner: deps.api.addr_validate(&msg.owner)?,
        mint_cost: msg.mint_cost,
        base_token_uri: msg.base_token_uri,
        limit_per_address: msg.limit_per_address,
        nft_limit: msg.nft_limit,
        payment_addr: msg.payment_addr,
        start_time: msg.start_time,
        name: msg.name.clone(),
        symbol: msg.symbol.clone(),
        uri: msg.uri.clone(),
    };

    // TODO: check mint_cost denom?

    CONFIG.save(deps.storage, &cfg)?;
    MINTABLE_NFT_LIMIT.save(deps.storage, &msg.nft_limit)?;

    // TODO: add launchpad status

    // Creating a message to instantiate a new bs721 contract
    let submessage = SubMsg {
        msg: WasmMsg::Instantiate {
            admin: Some(cfg.owner.to_string()),
            code_id: INSTANTIATE_BS721_CODE_ID,
            msg: to_binary(&Cw721InstantiateMsg {
                name: msg.name.clone(),
                symbol: msg.symbol.clone(),
                minter: env.contract.address.to_string(),
                uri: msg.uri.clone(),
            })?,
            funds: vec![],
            label: format!("BS721-{}", msg.name.trim()),
        }
        .into(),
        gas_limit: None,
        id: INSTANTIATE_REPLY_ID,
        reply_on: ReplyOn::Success,
    };

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("contract_name", CONTRACT_NAME)
        .add_attribute("contract_version", CONTRACT_VERSION)
        .add_attribute("sender", info.sender.clone())
        .add_submessage(submessage))
}

// #[cfg_attr(not(feature = "library"), entry_point)]
// pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
//     match msg {

//     }
// }

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response> {
    match msg.id {
        INSTANTIATE_REPLY_ID => handle_instantiate_reply(deps, msg),
        id => Err(StdError::generic_err(format!("Unknown reply id: {}", id))),
    }
}

fn handle_instantiate_reply(deps: DepsMut, msg: Reply) -> StdResult<Response> {
    // Handle the msg data and save the contract address
    // See: https://github.com/CosmWasm/cw-plus/blob/main/packages/utils/src/parse_reply.rs
    let response = parse_reply_instantiate_data(msg);
    match response {
        Ok(res) => {
            let nft_address = res.contract_address;
            NFT_ADDRESS.save(deps.storage, &Addr::unchecked(nft_address.clone()))?;
            Ok(Response::default()
                .add_attribute("action", "instantiate_bs721_reply")
                .add_attribute("nft_address", nft_address))
        }
        Err(_) => Err(StdError::generic_err(format!("Instantiate contract error"))),
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        coins,
        testing::{mock_dependencies, mock_env, mock_info},
        Api, Coin, Timestamp, Uint128,
    };

    use super::*;

    #[test]
    fn instantiate_test() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let creator_info = mock_info("creator", &coins(1000, "ubtsg"));

        let create_msg = InstantiateMsg {
            name: "test".to_string(),
            uri: Some("ipfs://...".to_string()),
            symbol: "test".to_string(),
            owner: creator_info.sender.to_string(),
            mint_cost: Coin {
                denom: "ubtsg".to_string(),
                amount: Uint128::new(100),
            },
            limit_per_address: 5,
            nft_limit: 10,
            payment_addr: None,
            base_token_uri: "ipfs://....".to_string(),
            start_time: Timestamp::from_nanos(166568230000000000),
        };

        let create_res =
            instantiate(deps.as_mut(), env.clone(), creator_info.clone(), create_msg).unwrap();
        assert_eq!(0, create_res.messages.len());
    }
}
