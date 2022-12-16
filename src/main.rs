mod indexer;
use crate::indexer::*;
use std::str::FromStr;

mod types;
use types::*;
use web3::{
    futures::{future, StreamExt},
    types::H160,
};

#[tokio::main]
async fn main() -> web3::contract::Result<()> {
    let _ = env_logger::try_init();
    let mut threads = Vec::new();

    // init socket
    let ws_url = "wss://eth-goerli.g.alchemy.com/v2/MV1WUqbDLAUTDyRfl_SquRlnjL64NfQr".to_string();
    let http_url =
        "https://eth-goerli.g.alchemy.com/v2/u8vzogVpxcy5OZmLdw1SVsgpMKTN-YCc".to_string();
    let web3 = indexer::get_websocket(&ws_url).await.unwrap();

    // init subscription
    let recipe_factory_address = "CAF3809F289eC0529360604dD8a53B55c94646F2";
    let contract = H160::from_str(recipe_factory_address).unwrap();
    let filter = get_filter(contract);
    let sub = web3.eth_subscribe().subscribe_logs(filter).await.unwrap();

    // init the indexing and launch the first listeners based on ongoing recipes
    init_main_indexer(&http_url, recipe_factory_address)
        .await
        .unwrap();
    let db = lfb_back::MongoRep::init("mongodb://localhost:27017/".to_string(), "lfb").unwrap();
    let recipes = db.get_recipes_ongoing().unwrap();
    recipes.into_iter().for_each(|x| {
        let db = lfb_back::MongoRep::init("mongodb://localhost:27017/".to_string(), "lfb").unwrap();
        let ws_clone = ws_url.clone();
        threads.push(tokio::spawn(async {
            sub_to_event(x.address, ws_clone, db).await
        }))
    });

    sub.for_each(|log| {
        let l = log.unwrap();
        let mut event = RecipeFactoryEventData::from_log(&l);
        let db = lfb_back::MongoRep::init("mongodb://localhost:27017/".to_string(), "lfb").unwrap();
        println!("Address received in contract was {}", event.recipe_address);
        db.add_recipe(
            &event.recipe_address,
            event.ingredients.iter_mut().map(|x| x.as_str()).collect(),
            l.block_number.unwrap_or_default().as_u64() as i64,
        )
        .unwrap();

        // TODO where is the topic coming from ?
        let ws_clone = ws_url.clone();

        let s = tokio::spawn(async { sub_to_event(event.recipe_address, ws_clone, db).await });
        threads.push(s);
        future::ready(())
    })
    .await;

    for thread in threads {
        thread.await.unwrap();
    }

    Ok(())
}
