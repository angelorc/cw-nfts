use crate::error::ContractError;
use crate::msg::{InstantiateMsg, ConfigResponse, QueryMsg, ExecuteMsg, StageResponse, Stage};
use crate::state::{Config, CONFIG, NFT_ADDRESS, SEED, NFT_POSITIONS, STAGES, STAGE_COUNTER, STAGE_REMAINING, STAGES_REMAINING};

use cosmwasm_std::{entry_point, to_binary, Addr, DepsMut, Env, MessageInfo, Reply, ReplyOn, Response, StdResult, SubMsg, WasmMsg, Deps, Binary, coin, Order, CosmosMsg, Empty, BankMsg, Storage, Uint128};
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
    pub position: u128,
    pub token_id: u128,
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let stages = msg.stages.clone();
    if stages.len() == 0 {
        return Err(ContractError::StageRequired {  })
    }

    if msg.seller_fee.clone() > MAX_SELLER_FEE {
        return Err(ContractError::SellerFeeBasisPointsTooHigh { })
    }

    let cfg = Config {
        admin: deps.api.addr_validate(&msg.admin)?,
        base_token_uri: msg.base_token_uri.clone(),
        payment_address: maybe_addr(deps.api, msg.payment_address.clone())?,
        name: msg.name.clone(),
        symbol: msg.symbol.clone(),
        collection_uri: msg.collection_uri.clone(),
        seller_fee_bps: msg.seller_fee.clone()
    };
    CONFIG.save(deps.storage, &cfg)?;

    let mut stages_supply = Uint128::from(0u128);
    let mut stage_counter = 0u8;

    for stage in stages.iter() {      
        if stage.total_amount == 0 {
            return Err(ContractError::StageInvalidSupply { })
        }

        stage_counter = stage_counter + 1u8;

        stages_supply = stages_supply + Uint128::from(stage.total_amount);
        
        STAGES.save(deps.storage, stage_counter.clone(), &stage.clone())?;
        STAGE_REMAINING.save(deps.storage, stage_counter.clone(), &Uint128::from(stage.total_amount))?;
    }

    STAGES_REMAINING.save(deps.storage, &stages_supply)?;
    STAGE_COUNTER.save(deps.storage, &stage_counter)?;

    // Store default seed
    let default_seed = [0_u8; 32];
    SEED.save(deps.storage, &default_seed)?;

    // Shuffle nft list
    let token_ids = random_nft_list(
        deps.storage,
        &env,
        deps.api
            .addr_validate(&msg.admin.to_string())?,
        (1u128..=u128::from(stages_supply)).collect::<Vec<u128>>(),
    )?;

    // Save token_ids map
    let mut nft_position = 1u128;
    for token_id in token_ids {
        NFT_POSITIONS.save(deps.storage, nft_position, &token_id)?;
        nft_position += 1u128;
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
    let nft_address = NFT_ADDRESS.load(deps.storage).unwrap_or(Addr::unchecked(""));

    if nft_address != "" {
        return Err(ContractError::BS721AlreadyLinked {})
    }

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
        ExecuteMsg::Mint { stage, proofs } => execute_mint(deps, env, info, stage, proofs),
    }
}

pub fn execute_mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    stage_id: u8,
    _proofs: Option<Vec<String>>
) -> Result<Response, ContractError> {
    let stage = STAGES.may_load(deps.storage, stage_id)?;

    match stage.clone() {
        Some(stage) => {
            // stage begun
            if let Some(start) = stage.start {
                if !start.is_triggered(&env.block) {
                    return Err(ContractError::StageNotBegun {  })
                }
            }

            // check not expired
            if let Some(expiration) = stage.expiration {
                if expiration.is_expired(&env.block) {
                    return Err(ContractError::StageExpired {  })
                }
            }

            // check remaining
            let stage_remaining = STAGE_REMAINING.load(deps.storage, stage_id)?;
            if stage_remaining.is_zero() {
                return Err(ContractError::StageSoldOut {  })
            }

            // check price
            if let Some(stage_price) = stage.price.clone() {
                let payment = may_pay(&info, &stage_price.denom).unwrap();
    
                if payment != &stage_price.amount {
                    return Err(ContractError::IncorrectPaymentAmount(
                        coin(payment.u128(), &stage_price.denom.clone()),
                        stage_price.clone(),
                    ));
                }
            }

            // TODO: add pause?
            // TODO: verify merkle_root

        },
        None => {
            return Err(ContractError::StageNotFound {  })
        },
    }

    let config = CONFIG.load(deps.storage)?;
    let nft_address = NFT_ADDRESS.load(deps.storage)?;
    let payment_address = config.payment_address.clone().unwrap_or(config.admin.clone()).to_string();

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
        seller_fee: config.seller_fee_bps,
        payment_address: Some(payment_address.clone())
    });

    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: nft_address.to_string(),
        msg: to_binary(&mint_msg)?,
        funds: vec![],
    });
    response = response.add_message(msg);

    // Remove mintable token position from map
    NFT_POSITIONS.remove(deps.storage, mintable_token_mapping.position.into());

    // Reduce stage remaining
    let mut stage_remaining = STAGE_REMAINING.load(deps.storage, stage_id.clone())?;
    stage_remaining = stage_remaining - Uint128::from(1u128);
    STAGE_REMAINING.save(deps.storage, stage_id.clone(), &stage_remaining)?;

    // Send funds to the owner
    if let Some(price) = stage.unwrap().price.clone() {
        let msg = BankMsg::Send {
            to_address: payment_address.clone(),
            amount: vec![price.clone()],
        };
        response = response.add_message(msg);
        response = response.add_attribute("price", price.to_string());
    } else {
        response = response.add_attribute("price", "0".to_string());
    }

    Ok(response
        .add_attribute("action", "execute_mint")
        .add_attribute("sender", info.sender.clone())
        .add_attribute("recipient", info.sender)
        .add_attribute("token_id", mintable_token_mapping.token_id.to_string())
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
    mut tokens: Vec<u128>,
) -> Result<Vec<u128>, ContractError> {
    let mut rng = generate_rng(storage, &env, sender)?;
    tokens.shuffle(&mut rng);

    Ok(tokens)
}

fn pick_random_nft(
    storage: &mut dyn Storage,
    env: &Env,
    sender: Addr,
) -> Result<TokenPositionMapping, ContractError> {
    let nft_remaining = STAGES_REMAINING.load(storage)?;
    
    let mut rng = generate_rng(storage, &env, sender)?;
    
    let next_random = rng.next_u32();
    
    let order = match next_random % 2 {
        1 => Order::Descending,
        _ => Order::Ascending,
    };

    let mut skip = 5u128;
    if skip > nft_remaining.u128() {
        skip = nft_remaining.u128()
    }

    skip = next_random as u128 % skip;
    
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
    let stages = STAGES
        .range(deps.storage, None, None, Order::Ascending)
        .map(map_stages)
        .collect::<StdResult<Vec<_>>>()?;

    Ok(ConfigResponse {
        nft_address: nft_address,
        admin: config.admin,
        payment_address: config.payment_address,
        name: config.name,
        symbol: config.symbol,
        collection_uri: config.collection_uri,
        base_token_uri: config.base_token_uri,
        seller_fee_bps: config.seller_fee_bps,
        stages: stages
    })
}

fn map_stages(item: StdResult<(u8, Stage)>) -> StdResult<StageResponse> {
    item.map(|(id, stage)| StageResponse {
        id,
        merkle_root: stage.merkle_root,
        start: stage.start,
        expiration: stage.expiration,
        price: stage.price,
        total_amount: stage.total_amount,
    })
}

#[cfg(test)]
mod tests {
    use crate::msg::Stage;

    use super::*;
    use prost::Message;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{SubMsgResult, SubMsgResponse, from_binary};

    // Type for replies to contract instantiate messes
    #[derive(Clone, PartialEq, Message)]
    struct MsgInstantiateContractResponse {
        #[prost(string, tag = "1")]
        pub contract_address: ::prost::alloc::string::String,
        #[prost(bytes, tag = "2")]
        pub data: ::prost::alloc::vec::Vec<u8>,
    }

    fn instantiate_msg() -> InstantiateMsg {
        InstantiateMsg {
            name: "test".to_string(),
            collection_uri: Some("ipfs://...".to_string()).clone(),
            symbol: String::from("test"),
            admin: String::from("creator"),
            stages: vec![
                Stage {
                    merkle_root: None,
                    start: None,
                    expiration: None,
                    price: Some(coin(100u128, "ubtsg".to_string())),
                    total_amount: 1u32
                },
            ],
            payment_address: Some(Addr::unchecked("payment_address").to_string()),
            base_token_uri: "ipfs://....".to_string(),
            seller_fee: 0,
        }
    }

    #[test]
    fn instantiate_test() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let creator_info = mock_info("creator", &[]);

        let init_msg = &instantiate_msg();

        let res = instantiate(deps.as_mut(), env.clone(), creator_info.clone(), init_msg.clone()).unwrap();

        assert_eq!(
            res.messages,
            vec![SubMsg {
                msg: WasmMsg::Instantiate {
                    code_id: INSTANTIATE_BS721_CODE_ID,
                    msg: to_binary(&Cw721InstantiateMsg {
                        name: init_msg.name.clone(),
                        symbol: init_msg.symbol.clone(),
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

        let instantiate_reply = MsgInstantiateContractResponse {
            contract_address: "nft_address".to_string(),
            data: vec![2u8; 32769],
        };
        let mut encoded_instantiate_reply =
            Vec::<u8>::with_capacity(instantiate_reply.encoded_len());
        instantiate_reply
            .encode(&mut encoded_instantiate_reply)
            .unwrap();

        let reply_msg = Reply {
            id: INSTANTIATE_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: Some(encoded_instantiate_reply.into()),
            }),
        };
        reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

        let query_msg = QueryMsg::Config {};
        let res = query(deps.as_ref(), mock_env(), query_msg.clone()).unwrap();
        let config: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(
            config,
            ConfigResponse {
                admin: creator_info.sender.clone(),
                name: init_msg.name.clone(),
                symbol: init_msg.symbol.clone(),
                collection_uri: init_msg.collection_uri.clone(),
                base_token_uri: init_msg.base_token_uri.clone(),
                seller_fee_bps: init_msg.seller_fee.clone(),
                payment_address: Some(Addr::unchecked("payment_address")),
                nft_address: Addr::unchecked("nft_address"),
                stages: vec![
                    StageResponse {
                        id: 1,
                        merkle_root: None,
                        start: None,
                        expiration: None,
                        price: Some(coin(100u128, "ubtsg".to_string())),
                        total_amount: 1u32
                    },
                ],
            }
        );
    }

    #[test]
    fn mint_test() {
        let mut deps = mock_dependencies();
        let info = mock_info("creator", &[]);
        let info_with_funds =  mock_info("creator", &[coin(100, "ubtsg".to_string())]);

        instantiate(deps.as_mut(), mock_env(), info.clone(), instantiate_msg()).unwrap();

        let instantiate_reply = MsgInstantiateContractResponse {
            contract_address: "nft_address".to_string(),
            data: vec![2u8; 32769],
        };

        let mut encoded_instantiate_reply = Vec::<u8>::with_capacity(instantiate_reply.encoded_len());
        instantiate_reply.encode(&mut encoded_instantiate_reply).unwrap();

        let reply_msg = Reply {
            id: INSTANTIATE_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: Some(encoded_instantiate_reply.into()),
            }),
        };

        reply(deps.as_mut(), mock_env(), reply_msg.clone()).unwrap();

        let mint_msg = ExecuteMsg::Mint{
            stage: 1u8,
            proofs: None
        };

        // free_mint, should mint one and fail the others
        execute(deps.as_mut(), mock_env(), info_with_funds.clone(), mint_msg.clone()).unwrap();

        let err = execute(deps.as_mut(), mock_env(), info.clone(), mint_msg.clone()).unwrap_err();
        match err {
            ContractError::StageSoldOut {  } => {},
            e => panic!("unexpected error: {}", e),
        }
    }
}
