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
    let ws_url = dotenv::var("ALCHEMY_API_WSS_KEY").expect("ALCHEMY_API_WSS_KEY must be set.");
    let http_url =
        dotenv::var("ALCHEMY_API_HTTPS_KEY").expect("ALCHEMY_API_HTTPS_KEY must be set.");

    let mongo_uri = dotenv::var("MONGO_URI").expect("MONGO_URI must be set.");

    let recipe_factory_address = dotenv::var("FACTORY_CONTRACT_ADDRESS").unwrap();
    let web3 = indexer::get_websocket(&ws_url).await.unwrap();
    let contract = H160::from_str(&recipe_factory_address).unwrap();
    let mut threads = Vec::new();

    // init subscription
    let filter = get_filter(contract);
    let sub = web3.eth_subscribe().subscribe_logs(filter).await.unwrap();

    // init the indexing and launch the first listeners based on ongoing recipes
    init_main_indexer(mongo_uri.clone(), &http_url, &recipe_factory_address)
        .await
        .unwrap();
    let db = lfb_back::MongoRep::init(mongo_uri.clone(), "lfb").unwrap();
    let recipes = db.get_recipes_ongoing().unwrap();
    recipes.into_iter().for_each(|x| {
        let db = lfb_back::MongoRep::init(mongo_uri.clone(), "lfb").unwrap();
        let ws_clone = ws_url.clone();
        threads.push(tokio::spawn(async {
            sub_to_event(x.address, ws_clone, db).await
        }))
    });

    sub.for_each(|log| {
        let l = log.unwrap();
        let mut event = RecipeFactoryEventData::from_log(&l);
        let db = lfb_back::MongoRep::init(mongo_uri.clone(), "lfb").unwrap();
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
