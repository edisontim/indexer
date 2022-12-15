use crate::types::{AddedIngredientEvent, RecipeFactoryEventData};
use std::{str, str::FromStr, thread};
use thiserror::Error;
use web3::{
    futures::{future, StreamExt},
    transports::WebSocket,
    types::{BlockNumber, Filter, FilterBuilder, H160},
    Web3,
};

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
    let db = lfb_back::MongoRep::init("mongodb://localhost:27017/".to_string(), "lfb").unwrap();

    let ongoing_recipes = db.get_recipes_ongoing().map_err(IndexerError::from)?;
    // first part: index all the recipes
    let current_block = w.eth().block_number().await?.as_u64();
    if let Some(last_recipe) = ongoing_recipes
        .iter()
        .max_by(|&x, &y| x.last_block.cmp(&y.last_block))
    {
        for x in ((last_recipe.last_block as u64)..current_block).step_by(1001) {
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
            let mut events: Vec<RecipeFactoryEventData> = logs
                .into_iter()
                .map(|x| RecipeFactoryEventData::from_log(&x))
                .collect();
            events.iter_mut().for_each(|event| {
                db.add_recipe(
                    &event.recipe_address,
                    event.ingredients.iter_mut().map(|x| x.as_str()).collect(),
                    event.block,
                );
            });
        }
    }

    // second part: index all the ingredients for those recipes
    let ongoing_recipes = db.get_recipes_ongoing().map_err(IndexerError::from)?;
    if let Some(last_recipe) = ongoing_recipes
        .iter()
        .max_by(|&x, &y| x.last_block.cmp(&y.last_block))
    {
        for x in ((last_recipe.last_block as u64)..current_block).step_by(501) {
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
            let mut events: Vec<AddedIngredientEvent> = logs
                .into_iter()
                .map(|x| AddedIngredientEvent::from_log(&x))
                .collect();
            events.iter_mut().for_each(|event| {
                db.update_recipe(
                    &event.recipe_address,
                    &event.hash,
                    &event.owner,
                    event.block,
                );
            });
        }
    }
    // TODO filter out the recipes completed events based on the topic. Call update_recipe_completed if event == recipe completed.
    // TODO change the last updated block by querying from the mongo API function get_last_block
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
    // TODO init connection to the mongo
    let contract = H160::from_str(&address).unwrap();
    let filter = get_filter(contract);
    let sub = web3.eth_subscribe().subscribe_logs(filter).await.unwrap();
    println!("Subbed to : {}", address);
    // TODO loop until recipe is received
    // TODO check when sub dies and how to reconnect
    sub.for_each(|log| {
        let l = log.unwrap();
        let topic = String::from("0x") + &hex::encode(l.topics[0]);
        let addedIngredientTopic =
            "0x04483ec0c137383f9f0a636e1d0b03e0d7b301d6b964cf0338137a8d90e0a1dd".to_string();
        let recipeCompletedTopic =
            "0x1d413284edcd8d8e4e70583af8454c3010040b97c1f9c641d16e903bda7b9f6a".to_string();
        match topic {
            addedIngredientTopic => {
                let event = AddedIngredientEvent::from_log(&l);
                db.update_recipe(
                    &event.recipe_address,
                    &event.hash,
                    &event.owner,
                    event.block,
                )
                .unwrap();
            }
            recipeCompletedTopic => {
                db.update_recipe_completed(&address);
            }
            _ => (),
        }

        // TODO parse the log and add to the mongo
        future::ready(())
    })
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_init_indexer() {
        init_main_indexer(
            "https://eth-goerli.g.alchemy.com/v2/u8vzogVpxcy5OZmLdw1SVsgpMKTN-YCc",
            "CAF3809F289eC0529360604dD8a53B55c94646F2",
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_match_topic() {
        let db = lfb_back::MongoRep::init("mongodb://localhost:27017/".to_string(), "lfb").unwrap();
        let address = "0xc250ff18654e552e9f25deb7ccc53c83f484f5a6";
        let topic = "0x1d413284edcd8d8e4e70583af8454c3010040b97c1f9c641d16e903bda7b9f6a";
        let addedIngredientTopic =
            "0x04483ec0c137383f9f0a636e1d0b03e0d7b301d6b964cf0338137a8d90e0a1dd";
        println!("{}", addedIngredientTopic);
        let recipeCompletedTopic =
            "0x1d413284edcd8d8e4e70583af8454c3010040b97c1f9c641d16e903bda7b9f6a";
        println!("{}", recipeCompletedTopic);
        let res = match topic {
            addedIngredientTopic => {
                println!("Got into AddedIngredient");
                false
            }
            recipeCompletedTopic => {
                println!("got into RecipeCompleted");
                db.update_recipe_completed(&address).unwrap()
            }
            _ => {
                print!("got into default");
                false
            }
        };
        assert_eq!(res, true);
    }
}
