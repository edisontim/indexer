use hex;
use serde::{Deserialize, Serialize};
use web3::types::Log;

#[derive(Debug, Deserialize, Serialize)]

pub struct RecipeFactoryEventData {
    pub recipe_contract_address: String,
    pub ingredients: Vec<String>,
    pub block: i64,
}

impl RecipeFactoryEventData {
    pub fn from_log(log: &Log) -> RecipeFactoryEventData {
        let mut ingredients: Vec<String> = Vec::new();
        let raw_data = &log.data.0;
        // Decompose raw data into u8 chunks of 32 bytes
        let chunks: Vec<&[u8]> = raw_data.chunks(32).collect();

        // Index 0 of chunks contains the newly created recipe contract's address
        let recipe_contract_address = String::from("0x") + &hex::encode(chunks[0].to_vec())[24..];

        // Index 2 of chunks contains number of ingredients in the last byte
        let num_ingredients = chunks[2][31];

        // Index 3 to 3 + num_ingredients contains the ingredients of the recipe
        let start_index = 3;
        for i in start_index..(start_index + num_ingredients as usize) {
            let hash = String::from("0x") + &hex::encode(chunks[i].to_vec());
            ingredients.push(hash);
        }

        RecipeFactoryEventData {
            recipe_contract_address: recipe_contract_address,
            ingredients: ingredients,
            block: log.block_number.unwrap_or_default().as_u64() as i64,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use web3::types::{BlockNumber, FilterBuilder, H160};

    use super::*;

    #[tokio::test]
    async fn test_from_log() {
        let w = web3::Web3::new(
            web3::transports::Http::new(
                "https://eth-goerli.g.alchemy.com/v2/u8vzogVpxcy5OZmLdw1SVsgpMKTN-YCc",
            )
            .unwrap(),
        );
        let logs = w
            .eth()
            .logs(
                FilterBuilder::default()
                    .address(vec![H160::from_str(
                        "CAF3809F289eC0529360604dD8a53B55c94646F2",
                    )
                    .unwrap()])
                    .from_block(BlockNumber::from(8067358))
                    .to_block(BlockNumber::from(8067359))
                    .build(),
            )
            .await
            .unwrap();
        let event = RecipeFactoryEventData::from_log(&logs[0]);
        assert_eq!(
            "0x2546a136b764e25107308290965aa026a92704cf",
            event.recipe_contract_address
        );
        assert_eq!(
            vec![
                "0x22f641503eeabdc00566e27be38734b69b308fb8d725a6362f1185d5fde190d4",
                "0x8574ea6bd913dd9b95296e9e5cede2d361f64f9b4a2f641b5fae3a2948be331e",
                "0xa2f0e044fddcfc4c905eb4be7b29778eb3eb4d48e704aad40b566062c289e4fb",
                "0xb2dacfb513e6886d424ee604ea7845721326cee47be4155927ace20c7d9b5b28"
            ],
            event.ingredients
        );
        assert_eq!(8067358, event.block);
    }
}
