// Here we write unit tests for the coin selection module

use crate::coin_selection::*;
use crate::input_validation::types::*;
use bitcoin::{ScriptBuf, Txid};
use std::str::FromStr;

// Helper function to create a dummy validated utxo for testing
fn utxo(value: u64, script: ScriptType) -> ValidatedUtxo {
    ValidatedUtxo {
        txid: Txid::from_str("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
            .unwrap(),
        vout: 0,
        value_sats: value,
        script_pubkey: ScriptBuf::new(),
        script_type: script,
        address: None,
    }
}

// Helper function to create a dummy validated payment for testing
fn payment(value: u64) -> ValidatedPayment {
    ValidatedPayment {
        value_sats: value,
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

// Test for the case where there should be no change output created(send all)
#[test]
fn test_exact_match_no_change() {
    let utxos = vec![utxo(50000, ScriptType::P2WPKH)];
    let payments = vec![payment(49500)];

    let result = LargestFirst
        .select(&utxos, &payments, &change(), 1.0, 5)
        .unwrap();

    assert!(!result.change_included);
}

// Test for the case where change output should be created, and the change value is above the dust threshold
#[test]
fn test_change_created() {
    let utxos = vec![utxo(100000, ScriptType::P2WPKH)];
    let payments = vec![payment(50000)];

    let result = LargestFirst
        .select(&utxos, &payments, &change(), 5.0, 5)
        .unwrap();

    assert!(result.change_included);
    assert!(result.change_value >= 546);
}

// Test for the case where change output should not be created because the change value is below the dust threshold,
// and it should be added to the fee instead
#[test]
fn test_dust_change_becomes_fee() {
    let utxos = vec![utxo(50000, ScriptType::P2WPKH)];
    let payments = vec![payment(49400)];

    let result = LargestFirst
        .select(&utxos, &payments, &change(), 5.0, 5)
        .unwrap();

    assert!(!result.change_included);
}

// Test for the case where the total input value is  less than the total payment value, and it should
// return an error for insufficient funds
#[test]
fn test_insufficient_funds() {
    let utxos = vec![
        utxo(10000, ScriptType::P2WPKH),
        utxo(25000, ScriptType::P2TR),
    ];
    let payments = vec![payment(50000)];
    let result = LargestFirst.select(&utxos, &payments, &change(), 5.0, 5);

    assert!(result.is_err());
}

// Test where we check if we are following the input limit correctly
#[test]
fn test_max_inputs_limit_success() {
    let utxos = vec![
        utxo(30000, ScriptType::P2WPKH),
        utxo(30000, ScriptType::P2WPKH),
    ];

    let payments = vec![payment(20000)];
    let result = LargestFirst
        .select(&utxos, &payments, &change(), 5.0, 1)
        .unwrap();

    assert_eq!(result.selected_coins.len(), 1);
}

// Test where we check if we are following the input limit correctly, and we expect it to fail because the limit is too low
#[test]
fn test_max_inputs_limit_failure() {
    let utxos = vec![
        utxo(20000, ScriptType::P2WPKH),
        utxo(20000, ScriptType::P2WPKH),
    ];

    let payments = vec![payment(35000)];
    let result = LargestFirst.select(&utxos, &payments, &change(), 5.0, 1);

    assert!(result.is_err());
}

// Test where we check if we are selecting an optimal input(p2tr preferred over p2wpkh) when we have a choice,
// and we expect it to select the p2tr input because it is more cost effective
#[test]
fn test_p2tr_preferred_over_p2wpkh() {
    let utxos = vec![
        utxo(60000, ScriptType::P2WPKH),
        utxo(60000, ScriptType::P2TR),
    ];

    let payments = vec![payment(30000)];

    let result = LargestFirst
        .select(&utxos, &payments, &change(), 5.0, 1)
        .unwrap();

    assert_eq!(result.selected_coins[0].script_type, ScriptType::P2TR);
}
