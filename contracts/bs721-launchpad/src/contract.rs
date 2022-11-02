use crate::error::ContractError;
use crate::msg::{InstantiateMsg, ConfigResponse, QueryMsg, ExecuteMsg};
use crate::state::{Config, CONFIG, NFT_ADDRESS, SEED, NFT_REMAINING, NFT_POSITIONS};

use cosmwasm_std::{entry_point, to_binary, Addr, DepsMut, Env, MessageInfo, Reply, ReplyOn, Response, StdResult, SubMsg, WasmMsg, Deps, Binary, coin, Order, CosmosMsg, Empty, BankMsg, Storage};
use cw2::set_contract_version;
use bs721_base::{InstantiateMsg as Cw721InstantiateMsg, Extension, MintMsg, ExecuteMsg as Cw721ExecuteMsg };
use cw_utils::{parse_reply_instantiate_data, maybe_addr, may_pay};
use rand::seq::SliceRandom;
use rand_chacha::ChaCha20Rng;
use rand_core::{RngCore, SeedableRng};
use sha2::{Digest, Sha256};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:bs721-launchpad";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANTIATE_BS721_CODE_ID: u64 = 40;
const INSTANTIATE_REPLY_ID: u64 = 1;

const MAX_SELLER_FEE: u16 = 10000; // mean 100.00%

pub struct TokenPositionMapping {
    pub position: u32,
    pub token_id: u32,
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Check max seller fee
    if msg.seller_fee > MAX_SELLER_FEE {
        return Err(ContractError::SellerFeeBasisPointsTooHigh { })
    }

    if msg.price.amount.is_zero() {
        return Err(ContractError::InvalidUnitPrice {});
    }

    if msg.nft_limit == 0 {
        return Err(ContractError::InvalidNftLimit {});
    }

    let cfg = Config {
        admin: deps.api.addr_validate(&msg.admin)?,
        price: msg.price.clone(),
        base_token_uri: msg.base_token_uri.clone(),
        payment_address: maybe_addr(deps.api, msg.payment_address.clone())?,
        name: msg.name.clone(),
        symbol: msg.symbol.clone(),
        collection_uri: msg.collection_uri.clone(),
        seller_fee_bps: msg.seller_fee.clone()
    };


    CONFIG.save(deps.storage, &cfg)?;
    NFT_REMAINING.save(deps.storage, &msg.nft_limit)?;

    // Store default seed
    let default_seed = [0_u8; 32];
    SEED.save(deps.storage, &default_seed)?;

    // Shuffle nft list
    let token_ids = random_nft_list(
        deps.storage,
        &env,
        deps.api
            .addr_validate(&msg.admin.to_string())?,
        (1..=msg.nft_limit).collect::<Vec<u32>>(),
    )?;

    // Save token_ids map
    let mut nft_position = 1;
    for token_id in token_ids {
        NFT_POSITIONS.save(deps.storage, nft_position, &token_id)?;
        nft_position += 1;
    }

    // Creating a message to instantiate a new bs721 contract
     let sub_msg = SubMsg {
        msg: WasmMsg::Instantiate {
            code_id: INSTANTIATE_BS721_CODE_ID,
            msg: to_binary(&Cw721InstantiateMsg {
                name: msg.name.clone(),
                symbol: msg.symbol.clone(),
                minter: env.contract.address.to_string(),
                uri: msg.collection_uri.clone(),
            })?,
            funds: info.funds,
            admin: Some(msg.admin.to_string()),
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
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Mint {} => execute_mint(deps, env, info),
    }
}

pub fn execute_mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let nft_address = NFT_ADDRESS.load(deps.storage)?;
    let payment_address = config.payment_address.clone().unwrap_or(config.admin.clone()).to_string();
    let nft_remaining = NFT_REMAINING.load(deps.storage)?;

    if nft_remaining == 0 {
        return Err(ContractError::CollectionSoldOut {  })
    }

    // Check payment
    let payment = may_pay(&info, &config.price.denom).unwrap();
    if payment != config.price.amount {
        return Err(ContractError::IncorrectPaymentAmount(
            coin(payment.u128(), &config.price.denom.clone()),
            config.price.clone(),
        ));
    }

    let mintable_token_mapping = pick_random_nft(deps.storage, &env, info.sender.clone())?;

    // Create mint msgs
    let mut response = Response::new();

    let mint_msg = Cw721ExecuteMsg::<Extension, Empty>::Mint(MintMsg::<Extension> {
        token_id: mintable_token_mapping.token_id.to_string(),
        token_uri: Some(format!(
            "{}/{}",
            config.base_token_uri, mintable_token_mapping.token_id
        )),
        owner: info.sender.clone().to_string(),
        extension: None,
        seller_fee: 0u16,
        payment_address: Some(payment_address.clone())
    });

    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: nft_address.unwrap().to_string(),
        msg: to_binary(&mint_msg)?,
        funds: vec![],
    });
    response = response.add_message(msg);

    // Remove mintable token position from map
    NFT_POSITIONS.remove(deps.storage, mintable_token_mapping.position);

    // Decrement nft limit
    NFT_REMAINING.save(deps.storage, &(nft_remaining - 1))?;

    // Send funds to the owner
    let msg = BankMsg::Send {
        to_address: payment_address.clone(),
        amount: vec![config.price.clone()],
    };
    response = response.add_message(msg);

    Ok(response
        .add_attribute("action", "execute_mint")
        .add_attribute("sender", info.sender.clone())
        .add_attribute("recipient", info.sender)
        .add_attribute("token_id", mintable_token_mapping.token_id.to_string())
        .add_attribute("price", config.price.to_string())
    )

}

fn generate_rng(
    storage: &mut dyn Storage,
    env: &Env,
    sender: Addr
) -> Result<ChaCha20Rng, ContractError> {
    let seed = SEED.load(storage)?;

    let tx_index = if let Some(tx) = &env.transaction {
        tx.index
    } else {
        0
    };

    let mut new_seed = seed.to_vec();
    new_seed.extend(env.block.chain_id.as_bytes().to_vec());
    new_seed.extend(&env.block.height.to_be_bytes());
    new_seed.extend(sender.as_bytes());
    new_seed.extend(tx_index.to_be_bytes());

    SEED.save(storage, &Sha256::digest(&new_seed).into())?;

    let rng = ChaCha20Rng::from_seed(seed);

    Ok(rng)
}

fn random_nft_list(
    storage: &mut dyn Storage,
    env: &Env,
    sender: Addr,
    mut tokens: Vec<u32>,
) -> Result<Vec<u32>, ContractError> {
    let mut rng = generate_rng(storage, &env, sender)?;
    tokens.shuffle(&mut rng);

    Ok(tokens)
}

fn pick_random_nft(
    storage: &mut dyn Storage,
    env: &Env,
    sender: Addr,
) -> Result<TokenPositionMapping, ContractError> {
    let nft_remaining = NFT_REMAINING.load(storage)?;
    
    let mut rng = generate_rng(storage, &env, sender)?;
    
    let next_random = rng.next_u32();
    
    let order = match next_random % 2 {
        1 => Order::Descending,
        _ => Order::Ascending,
    };

    let mut skip = 5;
    if skip > nft_remaining {
        skip = nft_remaining
    }

    skip = next_random % skip;
    
    let position = NFT_POSITIONS
        .keys(storage, None, None, order)
        .skip(skip as usize)
        .take(1)
        .collect::<StdResult<Vec<_>>>()?[0];

    let token_id = NFT_POSITIONS.load(storage, position)?;

    Ok(TokenPositionMapping { position, token_id })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    let nft_address = NFT_ADDRESS.load(deps.storage)?;
    let nft_remaining = NFT_REMAINING.load(deps.storage)?;

    Ok(ConfigResponse {
        nft_address: nft_address,
        admin: config.admin,
        payment_address: config.payment_address,
        price: config.price,
        name: config.name,
        symbol: config.symbol,
        collection_uri: config.collection_uri,
        base_token_uri: config.base_token_uri,
        nft_remaining: nft_remaining,
        seller_fee_bps: 0,
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
            collection_uri: Some("ipfs://...".to_string()).clone(),
            symbol: String::from("test"),
            admin: creator_info.sender.to_string(),
            price: Coin {
                denom: "ubtsg".to_string(),
                amount: Uint128::new(100),
            },
            nft_limit: 10,
            payment_address: None,
            base_token_uri: "ipfs://....".to_string(),
            seller_fee: 0,

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
