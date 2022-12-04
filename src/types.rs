use std::str::FromStr;

use serde::{Deserialize, Serialize};
use web3::types::{Log, H256};

#[derive(Debug, Deserialize, Serialize)]

pub struct RecipeFactoryEventData {
    pub recipe_contract_address: H256,
    pub ingredients: Vec<H256>,
}
pub struct AddedIngredientEvent {
    pub topic: String,
    pub hash: String,
    pub owner: String,
    pub block_number: i64,
}

impl RecipeFactoryEventData {
    pub fn from_raw_bytes(raw_data: &[u8]) -> RecipeFactoryEventData {
        let mut ingredients: Vec<H256> = Vec::new();
        // Decompose raw data into u8 dataChunks of 32 bytes
        let dataChunks: Vec<&[u8]> = raw_data.chunks(32).collect();

        // Index 0 of dataChunks contains the newly created recipe contract's address
        let recipe_contract_address =
            web3::types::H256::from_slice(dataChunks[0].try_into().unwrap());

        // Index 2 of dataChunks contains number of ingredients in the last byte
        let num_ingredients = dataChunks[2][31];

        // Index 3 to 3 + num_ingredients contains the ingredients of the recipe
        let start_index = 3;
        for i in start_index..(start_index + num_ingredients as usize) {
            let hex = web3::types::H256::from_slice(dataChunks[i].try_into().unwrap());
            ingredients.push(hex);
        }

        RecipeFactoryEventData {
            recipe_contract_address: recipe_contract_address,
            ingredients: ingredients,
        }
    }
}

impl AddedIngredientEvent {
    pub fn from_raw_bytes(l: Log) -> AddedIngredientEvent {
        let topic = l.topics[0];
        let dataChunks: Vec<&[u8]> = l.data.0.chunks(32).collect();
        let owner = String::from_str("0x").unwrap()
            + &format![
                "{:#x}",
                web3::types::H256::from_slice(dataChunks[1].try_into().unwrap())
            ][26..];
        let hash = format![
            "{:#x}",
            web3::types::H256::from_slice(dataChunks[0].try_into().unwrap())
        ];
        AddedIngredientEvent {
            hash: hash,
            owner: owner,
            topic: topic.to_string(),
            block_number: l.block_number.unwrap_or_default().as_u64() as i64,
        }
    }
}
