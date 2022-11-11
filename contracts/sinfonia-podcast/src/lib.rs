mod state;
mod msg;
mod query;

use std::time::Duration;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Empty, Timestamp};
use cw2::set_contract_version;
pub use cw721_base::{ContractError, MintMsg, MinterResponse};

use crate::msg::SiPodcastQueryMsg;
use crate::query::query_podcast_info;

use crate::state::{Podcast, PODCAST};
use crate::msg::InstantiateMsg;

// Version info for migration
const CONTRACT_NAME: &str = "crates.io:sinfonia-podcast";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cw_serde]
pub struct ItunesEpisode {
    pub order: Option<u32>,
    pub duration: Option<Duration>,
}

#[cw_serde]
pub struct EpisodeEnclosure {
    pub url: String,
    pub media_type: String,
}

#[cw_serde]
pub struct Episode {
    pub guid: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub pubdate: Option<Timestamp>,
    pub itunes: ItunesEpisode,
    pub enclosure: EpisodeEnclosure,
}

pub type Extension = Option<Episode>;

pub type SiPodcastContract<'a> = cw721_base::Cw721Contract<'a, Extension, Empty, Empty, SiPodcastQueryMsg>;
pub type ExecuteMsg = cw721_base::ExecuteMsg<Extension, Empty>;
pub type QueryMsg = cw721_base::QueryMsg<SiPodcastQueryMsg>;

#[cfg(not(feature = "library"))]
pub mod entry {
    use super::*;

    use cosmwasm_std::{entry_point, to_binary};
    use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

    #[entry_point]
    pub fn instantiate(
        mut deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> Result<Response, ContractError> {
        let init_msg = cw721_base::msg::InstantiateMsg {
            name: msg.title.clone(),
            minter: msg.minter.clone(),
            symbol: msg.symbol.clone(),
        };

        let res = SiPodcastContract::default().instantiate(deps.branch(), env, info, init_msg)?;

        let podcast = Podcast {
            description: msg.description,
            title: msg.title,
            link: msg.link,
        };

        PODCAST.save(deps.storage, &podcast)?;

        set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)
            .map_err(ContractError::Std)?;
        Ok(res)
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> Result<Response, ContractError> {
        SiPodcastContract::default().execute(deps, env, info, msg)
    }

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
        match msg {
            QueryMsg::Extension { msg } => match msg {
                SiPodcastQueryMsg::PodcastInfo { } => to_binary(&query_podcast_info(deps)?)
            },
            _ => SiPodcastContract::default().query(deps, env, msg)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::msg::ItunesDataMsg;

    use super::*;

    use cosmwasm_std::{testing::{mock_dependencies, mock_env, mock_info}, from_binary};
    use cw721::Cw721Query;

    const CREATOR: &str = "creator";

    #[test]
    fn use_podcast_extension() {
        let mut deps = mock_dependencies();
        let contract = SiPodcastContract::default();

        let info = mock_info(CREATOR, &[]);

        let init_msg = InstantiateMsg {
            title: "SpaceShips".to_string(),
            symbol: "SPACE".to_string(),
            minter: CREATOR.to_string(),
            description: "description".to_string(),
            link: Some("link".to_string()),
            lang: None,
            itunes: ItunesDataMsg {
                author: None,
                category: None,
                image: None,
                pod_type: Some("episodic".to_string()),
            }
        };

        entry::instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg)
            .unwrap();

        let token_id = "Enterprise";
        let mint_msg = MintMsg {
            token_id: token_id.to_string(),
            owner: "john".to_string(),
            token_uri: Some("https://starships.example.com/Starship/Enterprise.json".into()),
            extension: Some(Episode {
                guid: None,
                description: Some("Spaceship with Warp Drive".into()),
                title: Some("Starship USS Enterprise".to_string()),
                itunes: ItunesEpisode { 
                    order: None, 
                    duration: None 
                },
                enclosure: EpisodeEnclosure { 
                    url: "https://...".to_string(), 
                    media_type: "audio/mpeg".to_string() 
                },
                pubdate: None
            }),
        };
        let exec_msg = ExecuteMsg::Mint(mint_msg.clone());
        contract.execute(deps.as_mut(), mock_env(), info, exec_msg).unwrap();

        let res = contract.nft_info(deps.as_ref(), token_id.into()).unwrap();
        assert_eq!(res.token_uri, mint_msg.token_uri);
        assert_eq!(res.extension, mint_msg.extension);

        let query_msg = QueryMsg::Extension { msg: SiPodcastQueryMsg::PodcastInfo {  } };
        let query_res: Podcast = from_binary(&entry::query(deps.as_ref(), mock_env(), query_msg).unwrap()).unwrap();
        
        assert_eq!(query_res.title, "SpaceShips".to_string());
        assert_eq!(query_res.description, "description".to_string());
        assert_eq!(query_res.link, Some("link".to_string()));
    }
}