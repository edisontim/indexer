use lfb_back::*;
use std::str;
use std::str::FromStr;
use thiserror::Error;
use web3::{
    futures::{future, StreamExt},
    transports::WebSocket,
    types::{Filter, FilterBuilder, H160},
    Web3,
};

use crate::types::AddedIngredientEvent;

#[derive(Debug, Error)]
pub enum IndexerError {
    #[error("error with websocket")]
    WebsocketInitializationError(#[from] web3::Error),
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

pub async fn sub_to_recipe(contract_address: String, ws_url: String) -> String {
    let db = lfb_back::MongoRep::init("mongodb://localhost:27017/".to_string(), "lfb").unwrap();
    let web3 = get_websocket(&ws_url).await.unwrap();
    // Remove trailing zeroes from the address and encapsulte in H160 object for web3 library
    let contract_address = &contract_address[26..];
    let contract_bytes = H160::from_str(contract_address).unwrap();
    let filter = get_filter(contract_bytes);
    let sub = web3.eth_subscribe().subscribe_logs(filter).await.unwrap();

    sub.for_each(|log| {
        let ingredient_added_topic =
            String::from("0x04483ec0c137383f9f0a636e1d0b03e0d7b301d6b964cf0338137a8d90e0a1dd");
        // TODO parse the log and add to the mongo
        let l = log.unwrap();
        let topic = l.topics[0];
        let event_data = AddedIngredientEvent::from_raw_bytes(l);
        match topic.to_string() {
            ingredient_added_topic => {
                db.update_recipe(
                    &(String::from_str("0x").unwrap() + contract_address),
                    &event_data.hash,
                    &event_data.owner,
                    event_data.block_number,
                );
            }
            _ => {
                println!("Recipe was finished")
            }
        }
        future::ready(())
    })
    .await;
    // TODO loop until recipe is received
    // TODO check when sub dies and how to reconnect
    String::from_str(contract_address).unwrap()
}
