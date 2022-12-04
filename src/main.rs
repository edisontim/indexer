mod indexer;
use crate::indexer::*;
use hex_literal::hex;
use std::str::FromStr;
mod types;
use types::*;
use web3::{
    futures::{future, StreamExt},
    types::H160,
};

#[tokio::main]
async fn main() -> web3::contract::Result<()> {
    let recipe_factory_address = "CAF3809F289eC0529360604dD8a53B55c94646F2";
    let mut threads = Vec::new();

    let ws_url = "wss://eth-goerli.g.alchemy.com/v2/MV1WUqbDLAUTDyRfl_SquRlnjL64NfQr".to_string();
    let web3 = indexer::get_websocket(&ws_url).await.unwrap();
    let contract = H160::from_str(recipe_factory_address).unwrap();
    let filter = get_filter(contract);
    let sub = web3.eth_subscribe().subscribe_logs(filter).await.unwrap();

    sub.for_each(|log| {
        let db = lfb_back::MongoRep::init("mongodb://localhost:27017/".to_string(), "lfb").unwrap();
        let l = log.unwrap();
        let event = RecipeFactoryEventData::from_raw_bytes(&l.data.0);
        let contract_address = format!["{:#x}", event.recipe_contract_address];
        // contract address we get is 40 bytes with leading 0s and not 0x, db needs 0x and only 20 bytes
        let contract_address_db = &(String::from_str("0x").unwrap()
            + &(&format!["{:#x}", event.recipe_contract_address])[26..]);

        let hashes: Vec<String> = event
            .ingredients
            .iter()
            .map(|ing| format!["{:#x}", ing])
            .collect();
        db.add_recipe(
            contract_address_db,
            hashes.iter().map(|s| &**s).collect(),
            l.block_number.unwrap_or_default().as_u64() as i64,
        )
        .unwrap();

        // TODO where is the topic coming from ?
        let ws_clone = ws_url.clone();
        let s = tokio::spawn(async {
            {
                sub_to_recipe(contract_address, ws_clone).await
            }
        });
        threads.push(s);
        future::ready(())
    })
    .await;

    // threads.into_iter().map(|thread| {
    //     thread.join().expect("error joining the thread");
    // });
    Ok(())
}
