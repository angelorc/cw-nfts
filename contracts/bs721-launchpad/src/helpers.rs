use crate::{error::ContractError, state::{SEED, STAGES_REMAINING, NFT_POSITIONS}};
use cosmwasm_std::{StdResult, Storage, Env, Addr, Order};
use rand::{SeedableRng, seq::SliceRandom, RngCore};
use rand_chacha::ChaCha20Rng;
use sha2::{Sha256, Digest};

use crate::msg::{Stage, StageResponse};

pub struct TokenPositionMapping {
    pub position: u128,
    pub token_id: u128,
}

pub fn map_stages(item: StdResult<(u8, Stage)>) -> StdResult<StageResponse> {
    item.map(|(id, stage)| StageResponse {
        id,
        merkle_root: stage.merkle_root,
        start: stage.start,
        expiration: stage.expiration,
        price: stage.price,
        total_amount: stage.total_amount,
    })
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

pub fn random_nft_list(
    storage: &mut dyn Storage,
    env: &Env,
    sender: Addr,
    mut tokens: Vec<u128>,
) -> Result<Vec<u128>, ContractError> {
    let mut rng = generate_rng(storage, &env, sender)?;
    tokens.shuffle(&mut rng);

    Ok(tokens)
}

pub fn pick_random_nft(
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