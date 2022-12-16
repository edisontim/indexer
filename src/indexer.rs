use crate::types::{AddedIngredientEvent, RecipeFactoryEventData};
use std::{str, str::FromStr};
use thiserror::Error;
use web3::{
    futures::{future, StreamExt},
    transports::WebSocket,
    types::{BlockNumber, Filter, FilterBuilder, Log, H160},
    Web3,
};

const ADDED_INGREDIENT_TOPIC: &str =
    "0x04483ec0c137383f9f0a636e1d0b03e0d7b301d6b964cf0338137a8d90e0a1dd";
const RECIPE_COMPLETED_TOPIC: &str =
    "0x1d413284edcd8d8e4e70583af8454c3010040b97c1f9c641d16e903bda7b9f6a";

#[derive(Debug, Error)]
pub enum IndexerError {
    #[error("error with websocket")]
    WebsocketInitializationError(#[from] web3::Error),
    #[error("error with mongo repo")]
    MongoRepositoryError(#[from] lfb_back::MongoRepError),
}

pub async fn init_main_indexer(
    url: &str,
    factory_address: &str,
) -> Result<Vec<String>, IndexerError> {
    let w = web3::Web3::new(web3::transports::Http::new(url)?);
    let db = lfb_back::MongoRep::init("mongodb://localhost:27017/".to_string(), "lfb")?;

    // first part: index all the recipes
    let current_block = w.eth().block_number().await?.as_u64();
    let mut last_block = db.get_last_block()?;
    if last_block == 0 {
        last_block = std::env::var("FACTORY_CONTRACT_START_BLOCK")
            .expect("FACTORY_CONTRACT_START_BLOCK must be set.")
            .parse::<i64>()
            //if does not parse, something is wrong, need to panic
            .unwrap();
    }
    for x in ((last_block as u64)..current_block).step_by(1001) {
        // get logs
        let logs = w
            .eth()
            .logs(
                FilterBuilder::default()
                    .address(vec![H160::from_str(factory_address).unwrap()])
                    .from_block(BlockNumber::from(x))
                    .to_block(BlockNumber::from(x + 1000))
                    .build(),
            )
            .await?;
        // parse logs and add to db
        let events: Vec<RecipeFactoryEventData> = logs
            .into_iter()
            .map(|x| RecipeFactoryEventData::from_log(&x))
            .collect();
        for mut event in events {
            db.add_recipe(
                &event.recipe_address,
                event.ingredients.iter_mut().map(|x| x.as_str()).collect(),
                event.block,
            )?;
        }
    }

    // second part: index all the ingredients for those recipes
    let ongoing_recipes = db.get_recipes_ongoing().map_err(IndexerError::from)?;
    for x in ((last_block as u64)..current_block).step_by(501) {
        // get logs for the ingredients
        let recipes_addresses = ongoing_recipes
            .iter()
            .map(|x| H160::from_str(x.address.as_str()).unwrap())
            .collect::<Vec<H160>>();
        let logs = w
            .eth()
            .logs(
                FilterBuilder::default()
                    .address(recipes_addresses)
                    .from_block(BlockNumber::from(x))
                    .to_block(BlockNumber::from(x + 500))
                    .build(),
            )
            .await?;
        // parse logs and add to db
        logs.into_iter().for_each(|l| {
            let address = String::from("0x") + &hex::encode(l.address);
            match_log(&l, &address, &db);
        });
    }

    Ok(vec![])
}

pub async fn get_websocket(url: &str) -> Result<Web3<WebSocket>, IndexerError> {
    Ok(web3::Web3::new(
        web3::transports::WebSocket::new(url)
            .await
            .map_err(IndexerError::from)?,
    ))
}

pub fn get_filter(contract: H160) -> Filter {
    FilterBuilder::default()
        .address(vec![contract])
        .topics(None, None, None, None)
        .build()
}

pub async fn sub_to_event(address: String, ws_url: String, db: lfb_back::MongoRep) {
    let web3 = get_websocket(&ws_url).await.unwrap();
    let contract = H160::from_str(&address).unwrap();
    let filter = get_filter(contract);
    let sub = web3.eth_subscribe().subscribe_logs(filter).await.unwrap();
    println!("Subbed to : {}", address);
    // TODO TEST
    sub.take_while(|log| {
        let l = log.as_ref().unwrap();
        let is_added_ingredient = match_log(l, &address, &db);
        future::ready(is_added_ingredient)
    })
    .collect::<Vec<_>>()
    .await;
}

fn match_log(l: &Log, address: &str, db: &lfb_back::MongoRep) -> bool {
    let topic = String::from("0x") + &hex::encode(l.topics[0]);
    match topic {
        _ if topic == ADDED_INGREDIENT_TOPIC => {
            let event = AddedIngredientEvent::from_log(&l);
            db.update_recipe(
                &event.recipe_address,
                &event.hash,
                &event.owner,
                event.block,
            )
            .unwrap();
            true
        }
        _ if topic == RECIPE_COMPLETED_TOPIC => {
            db.update_recipe_completed(address).unwrap();
            false
        }
        _ => panic!("incorrect topic"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_init_indexer() {
        init_main_indexer(
            dotenv::var("ALCHEMY_API_HTTPS_KEY")
                .expect("ALCHEMY_API_HTTPS_KEY must be set")
                .as_str(),
            dotenv::var("FACTORY_CONTRACT_ADDRESS")
                .expect("FACTORY_CONTRACT_ADDRESS must be set")
                .as_str(),
        )
        .await
        .unwrap();
    }
}
