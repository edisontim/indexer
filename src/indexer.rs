use std::str::FromStr;

use std::thread;
use thiserror::Error;
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

async fn get_websocket(url: &str) -> Result<Web3<WebSocket>, IndexerError> {
    Ok(web3::Web3::new(
        web3::transports::WebSocket::new(url)
            .await
            .map_err(IndexerError::from)?,
    ))
}

fn get_filter(contract: H160) -> Filter {
    FilterBuilder::default()
        .address(vec![contract])
        .topics(Some(vec![contract.into()]), None, None, None)
        .build()
}

pub async fn spawn_indexer(address: &str, ws_url: String) {
    let web3 = get_websocket(&ws_url).await.unwrap();
    // TODO init connection to the mongo
    let contract = H160::from_str(address).unwrap();
    let filter = get_filter(contract);
    let sub = web3.eth_subscribe().subscribe_logs(filter).await.unwrap();
    let s = thread::spawn(move || {
        // TODO loop until recipe is received
        // TODO check when sub dies and how to reconnect
        sub.for_each(|log| {
            // TODO parse the log and add to the mongo
            future::ready(())
        })
    });
    s.join().expect("error joining the thread");
}
