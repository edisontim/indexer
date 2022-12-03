use core::num;

use serde::{Deserialize, Serialize};
use web3::{ethabi::Event, types::H256};

#[derive(Debug, Deserialize, Serialize)]

pub struct RecipeFactoryEventData {
    pub recipe_contract_address: String,
    pub ingredients: Vec<H256>,
}

impl RecipeFactoryEventData {
    pub fn from_raw_bytes(raw_data: Vec<u8>) -> RecipeFactoryEventData {
        let mut ingredients: Vec<H256> = Vec::new();
        // Decompose raw data into u8 chunks of 32 bytes
        let chunks: Vec<&[u8]> = raw_data.chunks(32).collect();

        // Index 0 of chunks contains the newly created recipe contract's address
        let recipe_contract_address =
            web3::types::H256::from_slice(chunks[0].try_into().unwrap()).to_string();

        // Index 2 of chunks contains number of ingredients in the last byte
        let num_ingredients = chunks[2][31];

        // Index 3 to 3 + num_ingredients contains the ingredients of the recipe
        let start_index = 3;
        for i in start_index..(start_index + num_ingredients as usize) {
            let hex = web3::types::H256::from_slice(chunks[i].try_into().unwrap());
            ingredients.push(hex);
        }

        RecipeFactoryEventData {
            recipe_contract_address: recipe_contract_address,
            ingredients: ingredients,
        }
    }
}
