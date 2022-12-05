use std::{str, str::FromStr, thread};
use thiserror::Error;
use types::RecipeFactoryEventData;
use web3::{
    futures::{future, StreamExt},
    transports::WebSocket,
    types::{Block, BlockNumber, Filter, FilterBuilder, H160},
    Web3,
};

use crate::types;

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
    let recipe_contracts: Vec<(&str, i64)> = ongoing_recipes
        .iter()
        .map(|x| (&*x.address, x.last_block))
        .collect::<Vec<(&str, i64)>>();

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
                .map(|x| types::RecipeFactoryEventData::from_log(&x))
                .collect();
            events.iter_mut().for_each(|event| {
                db.add_recipe(
                    &event.recipe_contract_address,
                    event.ingredients.iter_mut().map(|x| x.as_str()).collect(),
                    event.block,
                );
            });
        }
    }
    // TODO update recipe contracts from logs -> hash map.
    // TODO all recipe contracts (in one eth query or for each contract?), query logs for IngredientAdded.
    // If RecipeCompleted, remove from recipe contracts.
    Ok(vec![])
}

pub async fn get_websocket(url: &str) -> Result<Web3<WebSocket>, IndexerError> {
    Ok(web3::Web3::new(
        web3::transports::WebSocket::new(url)
            .await
            .map_err(IndexerError::from)?,
    ))
}

pub fn get_filter(contract: H160, topic: [u8; 32]) -> Filter {
    FilterBuilder::default()
        .address(vec![contract])
        .topics(Some(vec![topic.into()]), None, None, None)
        .build()
}

pub async fn sub_to_event(
    address: String,
    ws_url: String,
    topic: [u8; 32],
    db: lfb_back::MongoRep,
) {
    let web3 = get_websocket(&ws_url).await.unwrap();
    // TODO init connection to the mongo
    let contract = H160::from_str(&address).unwrap();
    let filter = get_filter(contract, topic);
    let sub = web3.eth_subscribe().subscribe_logs(filter).await.unwrap();
    let mut threads: Vec<thread::JoinHandle<()>> = Vec::new();

    // TODO loop until recipe is received
    // TODO check when sub dies and how to reconnect
    sub.for_each(|log| {
        // TODO parse the log and add to the mongo
        future::ready(())
    })
    .await;

    println!("Finished other function");
    threads.into_iter().map(|thread| {
        thread.join().expect("error joining the thread");
    });
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
}
