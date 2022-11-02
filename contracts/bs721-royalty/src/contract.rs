#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Decimal, BankMsg, to_binary};
use cw2::set_contract_version;
use cw4::{Cw4Contract, MemberResponse, MemberListResponse, Member};
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ConfigResponse};
use crate::state::{Config, CONFIG};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:bs721-royalty";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let cw4_address = Cw4Contract(deps.api.addr_validate(&msg.cw4_address).map_err(|_| {
        ContractError::InvalidCw4 {
            addr: msg.cw4_address.clone(),
        }
    })?);

    let weight = cw4_address.total_weight(&deps.querier)?;
    if weight == 0 {
        return Err(ContractError::InvalidCw4TotalWeight { weight });
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let cfg = Config { cw4_address };
    CONFIG.save(deps.storage, &cfg)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::PayMembers {  } => execute_pay_members(deps, env, info)
    }
}

pub fn execute_pay_members(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // TODO: add nonpayable
    
    let config = CONFIG.load(deps.storage)?;

    // only a member can pay to cw4 contract
    let weight = config
        .cw4_address
        .is_member(&deps.querier, &info.sender, None)?
        .ok_or(ContractError::Unauthorized {})?;
    if weight == 0 {
        return Err(ContractError::InvalidCw4TotalWeight { weight });
    }

    let total_weight = config.cw4_address.total_weight(&deps.querier)?;
    let members = config.cw4_address.list_members(&deps.querier, None, None)?;

    let balances = deps.querier.query_all_balances(env.contract.address)?;
    if balances.is_empty() {
        return Err(ContractError::NoCoins {});
    }

    let msgs = members
        .iter()
        .map(|member| {
            let ratio = Decimal::from_ratio(member.weight, total_weight);
            let amount = balances.iter().cloned().map(|mut c| {
                c.amount = c.amount * ratio;
                c
            })
            .collect();
            
            BankMsg::Send {
                to_address: member.addr.clone(),
                amount: amount,
            }
        })
        .collect::<Vec<_>>();

    Ok(Response::new()
        .add_attribute("action", "distribute")
        .add_messages(msgs))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::AllMembers { start_after, limit } => {
            to_binary(&query_all_members(deps, start_after, limit)?)
        }
        QueryMsg::Member { address } => to_binary(&query_member(deps, address)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;

    Ok(ConfigResponse { config })
}

fn query_member(deps: Deps, member: String) -> StdResult<MemberResponse> {
    let config = CONFIG.load(deps.storage)?;

    let member = deps.api.addr_validate(&member)?;
    let weight = config.cw4_address.is_member(&deps.querier, &member, None)?;

    Ok(MemberResponse { weight })
}

fn query_all_members(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<MemberListResponse> {
    let config = CONFIG.load(deps.storage)?;

    let members = config.cw4_address.list_members(&deps.querier, start_after, limit)?.into_iter()
        .map(|member| Member {
            addr: member.addr,
            weight: member.weight,
        })
        .collect();

    Ok(MemberListResponse { members })
}

#[cfg(test)]
mod tests {}
