use crate::error::ContractError;
use crate::msg::{InstantiateMsg, ConfigResponse, QueryMsg, ExecuteMsg};
use crate::state::{Config, CONFIG, NFT_ADDRESS, SEED, NFT_POSITIONS, STAGES};
use crate::validator::{validate_stages, validate_seller_fee};

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

    let msg_stages = msg.stages.clone();

    validate_seller_fee(msg.seller_fee.clone())?;   
    validate_stages(msg_stages.clone(), env.block.time.clone())?;

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
    STAGES.save(deps.storage, &msg_stages.clone().to_stages())?;

    // Store default seed
    let default_seed = [0_u8; 32];
    SEED.save(deps.storage, &default_seed)?;

    // Shuffle nft list
    let token_ids = random_nft_list(
        deps.storage,
        &env,
        deps.api
            .addr_validate(&msg.admin.to_string())?,
        (1..=msg_stages.total_supply()).collect::<Vec<u32>>(),
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
    let mut stages = STAGES.load(deps.storage)?;

    let current_stage = stages.current_stage(env.block.time)?;
    if current_stage.remaining() == 0 {
        return Err(ContractError::StageSoldOut {  })
    }

    // Check payment
    match current_stage.price.clone() {
        Some(price) => {

            let payment = may_pay(&info, &price.denom).unwrap();
    
            if payment != &price.amount {
                return Err(ContractError::IncorrectPaymentAmount(
                    coin(payment.u128(), &price.denom.clone()),
                    price.clone(),
                ));
            }
        }
        _ => {},
    };    

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
        contract_addr: nft_address.to_string(),
        msg: to_binary(&mint_msg)?,
        funds: vec![],
    });
    response = response.add_message(msg);

    // Remove mintable token position from map
    NFT_POSITIONS.remove(deps.storage, mintable_token_mapping.position);

    // Increment stage supply
    let mut stage = current_stage.clone();
    stage.supply = stage.supply + 1;

    if stage.label == "free_mint" {
        stages.free_mint = Some(stage)
    } else if stage.label == "pre_sale" {
        stages.pre_sale = Some(stage)
    } else {
        stages.public_sale = stage
    }

    STAGES.save(deps.storage, &stages)?;

    // Send funds to the owner
    match current_stage.price.clone() {
        Some(price) => {
            let msg = BankMsg::Send {
                to_address: payment_address.clone(),
                amount: vec![price.clone()],
            };
            response = response.add_message(msg);
            response = response.add_attribute("price", price.to_string());
        }
        None => {
            response = response.add_attribute("price", "0".to_string());
        },
    };  

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
    let stages = STAGES.load(storage)?;
    let nft_remaining = stages.remaining_supply();
    
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
    let stages = STAGES.load(deps.storage)?;

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

#[cfg(test)]
mod tests {
    use crate::msg::{StagesMsg, StageMsgWithPrice, StageMsgNoPrice};

    use super::*;
    use prost::Message;
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR},
        Coin, Timestamp, Uint128, SubMsgResult, SubMsgResponse, from_binary, BlockInfo, TransactionInfo, ContractInfo,
    };

    // Type for replies to contract instantiate messes
    #[derive(Clone, PartialEq, Message)]
    struct MsgInstantiateContractResponse {
        #[prost(string, tag = "1")]
        pub contract_address: ::prost::alloc::string::String,
        #[prost(bytes, tag = "2")]
        pub data: ::prost::alloc::vec::Vec<u8>,
    }

    pub fn mock_env_instantiate() -> Env {
        Env {
            block: BlockInfo {
                height: 12_345,
                time: Timestamp::from_nanos(1571797430000000000),
                chain_id: "cosmos-testnet-14002".to_string(),
            },
            transaction: Some(TransactionInfo { index: 3 }),
            contract: ContractInfo {
                address: Addr::unchecked(MOCK_CONTRACT_ADDR),
            },
        }
    }

    pub fn mock_env_free_mint() -> Env {
        Env {
            block: BlockInfo {
                height: 12_345,
                time: Timestamp::from_nanos(1571797450000000000),
                chain_id: "cosmos-testnet-14002".to_string(),
            },
            transaction: Some(TransactionInfo { index: 3 }),
            contract: ContractInfo {
                address: Addr::unchecked(MOCK_CONTRACT_ADDR),
            },
        }
    }

    pub fn mock_env_pre_sale() -> Env {
        Env {
            block: BlockInfo {
                height: 12_345,
                time: Timestamp::from_nanos(1571797480000000000),
                chain_id: "cosmos-testnet-14002".to_string(),
            },
            transaction: Some(TransactionInfo { index: 3 }),
            contract: ContractInfo {
                address: Addr::unchecked(MOCK_CONTRACT_ADDR),
            },
        }
    }

    pub fn mock_env_public_sale() -> Env {
        Env {
            block: BlockInfo {
                height: 12_345,
                time: Timestamp::from_nanos(1571797510000000000),
                chain_id: "cosmos-testnet-14002".to_string(),
            },
            transaction: Some(TransactionInfo { index: 3 }),
            contract: ContractInfo {
                address: Addr::unchecked(MOCK_CONTRACT_ADDR),
            },
        }
    }

    fn init_msg_full() -> InstantiateMsg {
        InstantiateMsg {
            name: "test".to_string(),
            collection_uri: Some("ipfs://...".to_string()).clone(),
            symbol: String::from("test"),
            admin: String::from("creator"),
            stages: StagesMsg { 
                free_mint: Some(StageMsgNoPrice {                        
                    start_date: Timestamp::from_nanos(1571797440000000000),
                    end_date:   Timestamp::from_nanos(1571797460000000000),
                    max_supply: 1,
                }),
                pre_sale: Some(StageMsgWithPrice {
                    start_date: Timestamp::from_nanos(1571797470000000000),
                    end_date:   Timestamp::from_nanos(1571797490000000000),
                    max_supply: 2,
                    price: Coin {
                        denom: "ubtsg".to_string(),
                        amount: Uint128::new(100),
                    }
                }),
                public_sale: StageMsgWithPrice {
                    start_date: Timestamp::from_nanos(1571797500000000000),
                    end_date:   Timestamp::from_nanos(1571797520000000000),
                    max_supply: 3,
                    price: Coin {
                        denom: "ubtsg".to_string(),
                        amount: Uint128::new(100),
                    }
                }
            },
            payment_address: Some(Addr::unchecked("payment_address").to_string()),
            base_token_uri: "ipfs://....".to_string(),
            seller_fee: 0,
        }
    }

    fn init_msg_free_public() -> InstantiateMsg {
        InstantiateMsg {
            name: "test".to_string(),
            collection_uri: Some("ipfs://...".to_string()).clone(),
            symbol: String::from("test"),
            admin: String::from("creator"),
            stages: StagesMsg { 
                free_mint: Some(StageMsgNoPrice {                        
                    start_date: Timestamp::from_nanos(1571797440000000000),
                    end_date:   Timestamp::from_nanos(1571797460000000000),
                    max_supply: 1,
                }),
                pre_sale: None,
                public_sale: StageMsgWithPrice {
                    start_date: Timestamp::from_nanos(1571797500000000000),
                    end_date:   Timestamp::from_nanos(1571797520000000000),
                    max_supply: 10,
                    price: Coin {
                        denom: "ubtsg".to_string(),
                        amount: Uint128::new(100),
                    }
                }
            },
            payment_address: None,
            base_token_uri: "ipfs://....".to_string(),
            seller_fee: 0,
        }
    }

    fn init_msg_pre_public() -> InstantiateMsg {
        InstantiateMsg {
            name: "test".to_string(),
            collection_uri: Some("ipfs://...".to_string()).clone(),
            symbol: String::from("test"),
            admin: String::from("creator"),
            stages: StagesMsg { 
                free_mint: None,
                pre_sale: Some(StageMsgWithPrice {
                    start_date: Timestamp::from_nanos(1571797470000000000),
                    end_date:   Timestamp::from_nanos(1571797490000000000),
                    max_supply: 1,
                    price: Coin {
                        denom: "ubtsg".to_string(),
                        amount: Uint128::new(100),
                    }
                }),
                public_sale: StageMsgWithPrice {
                    start_date: Timestamp::from_nanos(1571797500000000000),
                    end_date:   Timestamp::from_nanos(1571797520000000000),
                    max_supply: 10,
                    price: Coin {
                        denom: "ubtsg".to_string(),
                        amount: Uint128::new(100),
                    }
                }
            },
            payment_address: None,
            base_token_uri: "ipfs://....".to_string(),
            seller_fee: 0,
        }
    }

    fn init_msg_public() -> InstantiateMsg {
        InstantiateMsg {
            name: "test".to_string(),
            collection_uri: Some("ipfs://...".to_string()).clone(),
            symbol: String::from("test"),
            admin: String::from("creator"),
            stages: StagesMsg { 
                free_mint: None,
                pre_sale: None,
                public_sale: StageMsgWithPrice {
                    start_date: Timestamp::from_nanos(1571797500000000000),
                    end_date:   Timestamp::from_nanos(1571797520000000000),
                    max_supply: 10,
                    price: Coin {
                        denom: "ubtsg".to_string(),
                        amount: Uint128::new(100),
                    }
                }
            },
            payment_address: None,
            base_token_uri: "ipfs://....".to_string(),
            seller_fee: 0,
        }
    }

    #[test]
    fn instantiate_full_test() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let creator_info = mock_info("creator", &[]);

        let init_msg = &init_msg_full();

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
                stages: init_msg.stages.to_stages(),
            }
        );
    }

    #[test]
    fn instantiate_public_test() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let creator_info = mock_info("creator", &[]);

        let init_msg = &init_msg_public();

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
                payment_address: None,
                nft_address: Addr::unchecked("nft_address"),
                stages: init_msg.stages.to_stages(),
            }
        );
    }

    #[test]
    fn instantiate_free_public_test() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let creator_info = mock_info("creator", &[]);

        let init_msg = &init_msg_free_public();

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
                payment_address: None,
                nft_address: Addr::unchecked("nft_address"),
                stages: init_msg.stages.to_stages(),
            }
        );
    }

    #[test]
    fn instantiate_pre_public_test() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let creator_info = mock_info("creator", &[]);

        let init_msg = &init_msg_pre_public();

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
                payment_address: None,
                nft_address: Addr::unchecked("nft_address"),
                stages: init_msg.stages.to_stages(),
            }
        );
    }

    #[test]
    fn invalid_reply_id() {
        let mut deps = mock_dependencies();
        let info = mock_info("creator", &[]);

        let init_msg = &init_msg_full();

        instantiate(deps.as_mut(), mock_env(), info, init_msg.clone()).unwrap();

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
            id: 2,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: Some(encoded_instantiate_reply.into()),
            }),
        };

        let err = reply(deps.as_mut(), mock_env(), reply_msg).unwrap_err();
        match err {
            ContractError::UnknownReplyId {  } => {},
            e => panic!("unexpected error: {}", e),
        }
    }

    #[test]
    fn alreay_linked() {
        let mut deps = mock_dependencies();
        let info = mock_info("creator", &[]);

        let init_msg = &init_msg_full();

        instantiate(deps.as_mut(), mock_env(), info, init_msg.clone()).unwrap();

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

        let err = reply(deps.as_mut(), mock_env(), reply_msg).unwrap_err();
        match err {
            ContractError::BS721AlreadyLinked {  } => {},
            e => panic!("unexpected error: {}", e),
        }
    }

    #[test]
    fn mint_full_sold_out() {
        let mut deps = mock_dependencies();
        let info = mock_info("creator", &[]);
        let info_with_funds =  mock_info("creator", &[coin(100, "ubtsg".to_string())]);

        let init_msg = &init_msg_full();

        instantiate(deps.as_mut(), mock_env_instantiate(), info.clone(), init_msg.clone()).unwrap();

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

        reply(deps.as_mut(), mock_env_instantiate(), reply_msg.clone()).unwrap();

        let mint_msg = ExecuteMsg::Mint{};

        // free_mint, should mint one and fail the others
        execute(deps.as_mut(), mock_env_free_mint(), info.clone(), mint_msg.clone()).unwrap();
        let err = execute(deps.as_mut(), mock_env_free_mint(), info.clone(), mint_msg.clone()).unwrap_err();
        match err {
            ContractError::StageSoldOut {  } => {},
            e => panic!("unexpected error: {}", e),
        }

        // pre_mint, should mint two and fail the others
        execute(deps.as_mut(), mock_env_pre_sale(), info_with_funds.clone(), mint_msg.clone()).unwrap();
        execute(deps.as_mut(), mock_env_pre_sale(), info_with_funds.clone(), mint_msg.clone()).unwrap();
        let err = execute(deps.as_mut(), mock_env_pre_sale(), info_with_funds.clone(), mint_msg.clone()).unwrap_err();
        match err {
            ContractError::StageSoldOut {  } => {},
            e => panic!("unexpected error: {}", e),
        }

        // public_mint, should mint three and fail the others
        execute(deps.as_mut(), mock_env_public_sale(), info_with_funds.clone(), mint_msg.clone()).unwrap();
        execute(deps.as_mut(), mock_env_public_sale(), info_with_funds.clone(), mint_msg.clone()).unwrap();
        execute(deps.as_mut(), mock_env_public_sale(), info_with_funds.clone(), mint_msg.clone()).unwrap();
        let err = execute(deps.as_mut(), mock_env_public_sale(), info_with_funds, mint_msg).unwrap_err();
        match err {
            ContractError::StageSoldOut {  } => {},
            e => panic!("unexpected error: {}", e),
        }
    }

    #[test]
    fn mint_full() {
        let mut deps = mock_dependencies();
        let info = mock_info("creator", &[]);
        let info_with_funds =  mock_info("creator", &[coin(100, "ubtsg".to_string())]);

        let init_msg = &init_msg_full();

        instantiate(deps.as_mut(), mock_env_instantiate(), info.clone(), init_msg.clone()).unwrap();

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

        reply(deps.as_mut(), mock_env_instantiate(), reply_msg.clone()).unwrap();

        let mint_msg = ExecuteMsg::Mint{};
        
        execute(deps.as_mut(), mock_env_free_mint(), info.clone(), mint_msg.clone()).unwrap();
        execute(deps.as_mut(), mock_env_pre_sale(), info_with_funds.clone(), mint_msg.clone()).unwrap();
        execute(deps.as_mut(), mock_env_public_sale(), info_with_funds, mint_msg).unwrap();
    }

    #[test]
    fn mint_free_public() {
        let mut deps = mock_dependencies();
        let info = mock_info("creator", &[]);
        let info_with_funds =  mock_info("creator", &[coin(100, "ubtsg".to_string())]);

        let init_msg = &init_msg_free_public();

        instantiate(deps.as_mut(), mock_env_instantiate(), info.clone(), init_msg.clone()).unwrap();

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

        reply(deps.as_mut(), mock_env_instantiate(), reply_msg.clone()).unwrap();

        let mint_msg = ExecuteMsg::Mint{};
        
        execute(deps.as_mut(), mock_env_free_mint(), info.clone(), mint_msg.clone()).unwrap();
        
        let err = execute(deps.as_mut(), mock_env_pre_sale(), info_with_funds.clone(), mint_msg.clone()).unwrap_err();
        match err {
            ContractError::NoActiveStages {  } => {},
            e => panic!("unexpected error: {}", e),
        }

        execute(deps.as_mut(), mock_env_public_sale(), info_with_funds, mint_msg).unwrap();
    }

    #[test]
    fn mint_pre_public() {
        let mut deps = mock_dependencies();
        let info = mock_info("creator", &[]);
        let info_with_funds =  mock_info("creator", &[coin(100, "ubtsg".to_string())]);

        let init_msg = &init_msg_pre_public();

        instantiate(deps.as_mut(), mock_env_instantiate(), info.clone(), init_msg.clone()).unwrap();

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

        reply(deps.as_mut(), mock_env_instantiate(), reply_msg.clone()).unwrap();

        let mint_msg = ExecuteMsg::Mint{};
        
        let err = execute(deps.as_mut(), mock_env_free_mint(), info.clone(), mint_msg.clone()).unwrap_err();
        match err {
            ContractError::NoActiveStages {  } => {},
            e => panic!("unexpected error: {}", e),
        }
        
        execute(deps.as_mut(), mock_env_pre_sale(), info_with_funds.clone(), mint_msg.clone()).unwrap();

        execute(deps.as_mut(), mock_env_public_sale(), info_with_funds, mint_msg).unwrap();
    }

    #[test]
    fn mint_public() {
        let mut deps = mock_dependencies();
        let info = mock_info("creator", &[]);
        let info_with_funds =  mock_info("creator", &[coin(100, "ubtsg".to_string())]);

        let init_msg = &init_msg_public();

        instantiate(deps.as_mut(), mock_env_instantiate(), info.clone(), init_msg.clone()).unwrap();

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

        reply(deps.as_mut(), mock_env_instantiate(), reply_msg.clone()).unwrap();

        let mint_msg = ExecuteMsg::Mint{};
        
        let err = execute(deps.as_mut(), mock_env_free_mint(), info.clone(), mint_msg.clone()).unwrap_err();
        match err {
            ContractError::NoActiveStages {  } => {},
            e => panic!("unexpected error: {}", e),
        }
        
        let err = execute(deps.as_mut(), mock_env_pre_sale(), info_with_funds.clone(), mint_msg.clone()).unwrap_err();
        match err {
            ContractError::NoActiveStages {  } => {},
            e => panic!("unexpected error: {}", e),
        }

        execute(deps.as_mut(), mock_env_public_sale(), info_with_funds, mint_msg).unwrap();
    }

}
