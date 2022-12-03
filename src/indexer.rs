use super::types::RecipeFactoryEventData;
use lfb_back::*;
use std::str;
use std::thread;
use std::{str::FromStr, thread::JoinHandle};
use thiserror::Error;
use web3::ethabi::{Event, EventParam, Log, ParamType, RawLog};
use web3::{
    futures::{future, StreamExt},
    transports::WebSocket,
    types::{Filter, FilterBuilder, H160},
    Web3,
};

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
