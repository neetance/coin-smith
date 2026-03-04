// Here we write unit tests to check the PSBT generation logic in the unsigned_tx_builder module

use crate::coin_selection::*;
use crate::input_validation::types::*;
use crate::unsigned_tx_builder::build_unsigned_tx;
use base64::{Engine as _, engine::general_purpose};
use bitcoin::{ScriptBuf, Txid};
use std::str::FromStr;

// Helper function to create a dummy validated utxo for testing
fn utxo() -> ValidatedUtxo {
    ValidatedUtxo {
        txid: Txid::from_str("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc")
            .unwrap(),
        vout: 0,
        value_sats: 100000,
        script_pubkey: ScriptBuf::new(),
        script_type: ScriptType::P2WPKH,
        address: None,
    }
}

// Helper function to create a dummy validated payment for testing
fn payment() -> ValidatedPayment {
    ValidatedPayment {
        value_sats: 50000,
        script_pubkey_hex: ScriptBuf::new(),
        script_type: ScriptType::P2WPKH,
        address: None,
    }
}

// Helper function to create a dummy validated change for testing
fn change() -> ValidatedChange {
    ValidatedChange {
        script_pubkey_hex: ScriptBuf::new(),
        script_type: ScriptType::P2WPKH,
        address: None,
    }
}

// Test to check if the generated PSBT base64 string can be decoded successfully, and we expect it to decode without
// any error and the decoded bytes should have a reasonable length
#[test]
fn test_psbt_base64_decodes() {
    let coins = CoinSelectionResult {
        selected_coins: vec![utxo()],
        total_input_value: 100000,
        total_fee: 1000,
        change_included: true,
        change_value: 49000,
        vbytes: 200,
    };

    let result = build_unsigned_tx(
        String::from("stochastic_knapsack"),
        &[payment()],
        &coins,
        &change(),
        Some(true),
        None,
        Some(850000),
        5.0,
    )
    .unwrap();

    let decoded = general_purpose::STANDARD
        .decode(result.psbt_base64)
        .unwrap();

    assert!(decoded.len() > 10);
}

// Test to check if the RBF signaling is set correctly in the generated PSBT, and we expect it to be true since we set
// it to true in the input
#[test]
fn test_rbf_signaling() {
    let coins = CoinSelectionResult {
        selected_coins: vec![utxo()],
        total_input_value: 100000,
        total_fee: 1000,
        change_included: true,
        change_value: 49000,
        vbytes: 200,
    };

    let result = build_unsigned_tx(
        String::from("stochastic_knapsack"),
        &[payment()],
        &coins,
        &change(),
        Some(true),
        None,
        Some(850000),
        5.0,
    )
    .unwrap();

    assert!(result.rbf_signaling);
}

// Test to check if the locktime is set correctly in the generated PSBT, and we expect it to be set to 499999999 as we
// provided in the input, and it should be of type block height since it's below the threshold of 500 million
#[test]
fn test_locktime_block_height() {
    let coins = CoinSelectionResult {
        selected_coins: vec![utxo()],
        total_input_value: 100000,
        total_fee: 1000,
        change_included: true,
        change_value: 49000,
        vbytes: 200,
    };

    let result = build_unsigned_tx(
        String::from("stochastic_knapsack"),
        &[payment()],
        &coins,
        &change(),
        Some(true),
        Some(499999999),
        None,
        5.0,
    )
    .unwrap();

    assert_eq!(result.locktime_type, "block_height");
}

// Test to check if the locktime is set correctly in the generated PSBT, and we expect it to be set to 500000000 as we
// provided in the input, and it should be of type unix timestamp since it's above the threshold of 500 million
#[test]
fn test_locktime_unix_timestamp() {
    let coins = CoinSelectionResult {
        selected_coins: vec![utxo()],
        total_input_value: 100000,
        total_fee: 1000,
        change_included: true,
        change_value: 49000,
        vbytes: 200,
    };

    let result = build_unsigned_tx(
        String::from("stochastic_knapsack"),
        &[payment()],
        &coins,
        &change(),
        Some(true),
        Some(500000000),
        None,
        5.0,
    )
    .unwrap();

    assert_eq!(result.locktime_type, "unix_timestamp");
}
