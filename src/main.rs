mod indexer;
use crate::indexer::*;
use hex_literal::hex;
use lfb_back::*;
use std::thread;
use std::{borrow::Borrow, str::FromStr};

mod types;
use types::*;
use web3::{
    futures::{future, StreamExt},
    transports::WebSocket,
    types::{Filter, FilterBuilder, H160},
    Web3,
};

#[tokio::main]
async fn main() -> web3::contract::Result<()> {
    let _ = env_logger::try_init();
    let ws_url = "wss://eth-goerli.g.alchemy.com/v2/xzLGA007HrDWYtTHz---NktiNAOtTYM3".to_string();

    let recipe_factory_address = "CAF3809F289eC0529360604dD8a53B55c94646F2";
    let recipe_factory_topic =
        hex!["552ac3e7e359ade147cee1b49895f531576e6991306de88664d7fe6673b214ed"];

    let web3 = indexer::get_websocket(&ws_url).await.unwrap();
    // TODO init connection to the mongo
    let contract = H160::from_str(recipe_factory_address).unwrap();
    let filter = get_filter(contract, recipe_factory_topic);
    let sub = web3.eth_subscribe().subscribe_logs(filter).await.unwrap();
    let mut threads = Vec::new();

    sub.for_each(|log| {
        let event = RecipeFactoryEventData::from_raw_bytes(log.unwrap().data.0);
        let db = lfb_back::MongoRep::init("mongodb://localhost:27017/".to_string(), "lfb").unwrap();
        let contract_address = &event.recipe_contract_address;
        dbg!(contract_address);
        dbg!(event
            .ingredients
            .iter()
            .map(|ing| format!["{:#x}", ing])
            .collect::<Vec<String>>());
        // db.add_recipe(
        //     &event.recipe_contract_address,
        //     event
        //         .ingredients
        //         .into_iter()
        //         .map(|ing| format!["{:#x}", ing].as_ref())
        //         .collect(),
        // )
        // .unwrap();

        // TODO where is the topic coming from ?
        let ws_clone = ws_url.clone();

        let s = thread::spawn(move || {
            sub_to_event(
                event.recipe_contract_address,
                ws_clone,
                hex!["552ac3e7e359ade147cee1b49895f531576e6991306de88664d7fe6673b214ed"],
                db,
            )
        });
        threads.push(s);
        future::ready(())
    })
    .await;

    println!("Finished main function");
    Ok(())
}
