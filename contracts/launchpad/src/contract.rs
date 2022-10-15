use cosmwasm_std::{entry_point, to_binary, Addr, DepsMut, Env, MessageInfo, Reply, ReplyOn, Response, StdResult, SubMsg, WasmMsg, Deps, Binary};
use cw2::set_contract_version;
use cw721_base::InstantiateMsg as Cw721InstantiateMsg;
use cw_utils::{parse_reply_instantiate_data, maybe_addr};
use crate::error::ContractError;

use crate::msg::{InstantiateMsg, ConfigResponse, QueryMsg};
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
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let cfg = Config {
        owner: deps.api.addr_validate(&msg.owner)?,
        mint_cost: msg.mint_cost.clone(),
        base_token_uri: msg.base_token_uri.clone(),
        limit_per_address: msg.limit_per_address.clone(),
        nft_limit: msg.nft_limit.clone(),
        payment_addr: maybe_addr(deps.api, msg.payment_addr.clone())?,
        name: msg.name.clone(),
        symbol: msg.symbol.clone(),
        uri: msg.uri.clone(),
    };

    if msg.mint_cost.amount.is_zero() {
        return Err(ContractError::InvalidUnitPrice {});
    }

    if msg.nft_limit == 0 {
        return Err(ContractError::InvalidNftLimit {});
    }

    CONFIG.save(deps.storage, &cfg)?;
    MINTABLE_NFT_LIMIT.save(deps.storage, &msg.nft_limit)?;

    // TODO: add launchpad status

    // Creating a message to instantiate a new bs721 contract
     let sub_msg = SubMsg {
        msg: WasmMsg::Instantiate {
            code_id: INSTANTIATE_BS721_CODE_ID,
            msg: to_binary(&Cw721InstantiateMsg {
                name: msg.name.clone(),
                symbol: msg.symbol.clone(),
                minter: env.contract.address.to_string(),
                uri: msg.uri.clone(),
            })?,
            funds: info.funds,
            admin: Some(msg.owner.to_string()),
            label: format!("BS721-{}", msg.name.trim()),
        }
        .into(),
        id: INSTANTIATE_REPLY_ID,
        gas_limit: None,
        reply_on: ReplyOn::Success,
    };

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("contract_name", CONTRACT_NAME)
        .add_attribute("contract_version", CONTRACT_VERSION)
        .add_attribute("sender", info.sender.clone())
        .add_submessage(sub_msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        INSTANTIATE_REPLY_ID => handle_instantiate_reply(deps, msg),
        _ => Err(ContractError::UnknownReplyId {  }),
    }
}

fn handle_instantiate_reply(deps: DepsMut, msg: Reply) -> Result<Response, ContractError> {
    // TODO: is this necessary?
    /*let nft_address = NFT_ADDRESS.load(deps.storage)?;

    if nft_address != None {
        return Err(ContractError::BS721AlreadyLinked {})
    }*/

    let response = parse_reply_instantiate_data(msg);
    match response {
        Ok(res) => {
            NFT_ADDRESS.save(deps.storage, &Addr::unchecked(res.contract_address.clone()).into())?;
            Ok(Response::default()
                .add_attribute("action", "instantiate_bs721_reply")
                .add_attribute("nft_address", res.contract_address))
        }
        Err(_) => Err(ContractError::InstantiateContractError {  }),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&query_config(deps)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    let nft_address = NFT_ADDRESS.load(deps.storage)?;

    Ok(ConfigResponse {
        nft_address: nft_address,
        owner: config.owner,
        payment_addr: config.payment_addr,
        mint_cost: config.mint_cost,
        name: config.name,
        symbol: config.symbol,
        uri: config.uri,
        base_token_uri: config.base_token_uri,
        nft_limit: config.nft_limit,
        limit_per_address: config.limit_per_address,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message;
    use cosmwasm_std::{
        coins,
        testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR},
        Coin, Timestamp, Uint128,
    };

    // Type for replies to contract instantiate messes
    #[derive(Clone, PartialEq, Message)]
    struct MsgInstantiateContractResponse {
        #[prost(string, tag = "1")]
        pub contract_address: ::prost::alloc::string::String,
        #[prost(bytes, tag = "2")]
        pub data: ::prost::alloc::vec::Vec<u8>,
    }

    #[test]
    fn instantiate_test() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let ttt = Timestamp::from_nanos(166568230000000000);
        println!("time: {}", ttt);

        let creator_info = mock_info("creator", &coins(1000, "ubtsg"));

        let msg = InstantiateMsg {
            name: "test".to_string(),
            uri: Some("ipfs://...".to_string()).clone(),
            symbol: String::from("test"),
            owner: creator_info.sender.to_string(),
            mint_cost: Coin {
                denom: "ubtsg".to_string(),
                amount: Uint128::new(100),
            },
            limit_per_address: 5,
            nft_limit: 10,
            payment_addr: None,
            base_token_uri: "ipfs://....".to_string(),
        };

        let res = instantiate(deps.as_mut(), env.clone(), creator_info.clone(), msg.clone()).unwrap();
        //assert_eq!(0, create_res.messages.len());

        assert_eq!(
            res.messages,
            vec![SubMsg {
                msg: WasmMsg::Instantiate {
                    code_id: INSTANTIATE_BS721_CODE_ID,
                    msg: to_binary(&Cw721InstantiateMsg {
                        name: msg.name.clone(),
                        symbol: msg.symbol.clone(),
                        minter: MOCK_CONTRACT_ADDR.to_string(),
                        uri: Some("ipfs://...".to_string()),
                    })
                    .unwrap(),
                    funds: vec![],
                    admin: Some(creator_info.sender.to_string()),
                    label: String::from("BS721-test"),
                }
                .into(),
                id: INSTANTIATE_REPLY_ID,
                gas_limit: None,
                reply_on: ReplyOn::Success,
            }]
        );




    }
}
