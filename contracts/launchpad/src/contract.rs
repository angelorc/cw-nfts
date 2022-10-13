use cosmwasm_std::{entry_point, MessageInfo, DepsMut, Env, Response, StdResult, Coin, CosmosMsg, WasmMsg, to_binary};
use cw721::{Cw721ExecuteMsg, Cw721};

use crate::state::{Config, CONFIG};
use crate::msg::InstantiateMsg;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let cfg = Config {
        owner: deps.api.addr_validate(&msg.owner)?,
        mint_cost: msg.mint_cost,
        base_token_uri: msg.base_token_uri,
        limit_per_address: msg.limit_per_address,
        nft_limit: msg.nft_limit,
        payment_addr: msg.payment_addr,
        start_time: msg.start_time,  
        name: msg.name,
        symbol: msg.symbol,
        uri: msg.uri,      
    };

    // TODO: check mint_cost denom?

    CONFIG.save(deps.storage, &cfg)?;

    // TODO: add launchpad status
    
    Ok(Response::new().add_message(CosmosMsg::Wasm(WasmMsg::Instantiate {
        code_id: 2,
        label: "new collection ;)".to_string(),
        msg: to_binary(&Cw721ExecuteMsg::Instantiate {
            0: Cw721::InstantiateMsg {
                token_id,
                owner: reservation.owner,
                name,
                description,
                image,
            },
        })?,
        funds: vec![],
        admin: None,
    })),)
}

// #[cfg_attr(not(feature = "library"), entry_point)]
// pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
//     match msg {
        
//     }
// }

#[cfg(test)]
mod tests {
    use cosmwasm_std::{testing::{mock_dependencies, mock_env, mock_info}, coins, Uint128, Api, Timestamp};

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
            mint_cost: Coin { denom: "ubtsg".to_string(), amount: Uint128::new(100), },
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

