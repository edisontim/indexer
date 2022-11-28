use hex_literal::hex;
use std::time;
use web3::{
    contract::{Contract, Options},
    futures::{future, StreamExt},
    types::{FilterBuilder, H160},
};

#[tokio::main]
async fn main() -> web3::contract::Result<()> {
    let _ = env_logger::try_init();
    let web3 = web3::Web3::new(
        web3::transports::WebSocket::new(
            "wss://eth-goerli.g.alchemy.com/v2/xzLGA007HrDWYtTHz---NktiNAOtTYM3",
        )
        .await?,
    );
    let contract_bytes = hex!("93be29CdF291661D0d70e25EC283ce5E37c2f6e2");

    let contract = H160::from_slice(&contract_bytes);

    // Filter for Hello event in our contract
    let filter = FilterBuilder::default()
        .address(vec![contract])
        .topics(
            Some(vec![hex!(
                "d282f389399565f3671145f5916e51652b60eee8e5c759293a2f5771b8ddfd2e"
            )
            .into()]),
            None,
            None,
            None,
        )
        .build();

    let sub = web3.eth_subscribe().subscribe_logs(filter).await?;

    sub.for_each(|log| {
        println!("got log: {:?}", log);
        future::ready(())
    })
    .await;

    Ok(())
}
