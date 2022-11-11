use cosmwasm_std::{Deps, StdResult};

use crate::{state::{Podcast, PODCAST}};

pub fn query_podcast_info(deps: Deps) -> StdResult<Podcast> {
    let podcast = PODCAST.load(deps.storage)?;

    Ok(Podcast { 
        title: podcast.title, 
        description: podcast.description, 
        link: podcast.link 
    })
}