// Here we write unit tests to check the fee/change estimation logic in the fee estimator module

use crate::coin_selection::fee_estimator::*;
use crate::input_validation::types::*;
use bitcoin::{ScriptBuf, Txid};
use std::str::FromStr;

// Helper function to create a dummy validated utxo for testing
fn utxo(value: u64) -> ValidatedUtxo {
    ValidatedUtxo {
        txid: Txid::from_str("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
            .unwrap(),
        vout: 0,
        value_sats: value,
        script_pubkey: ScriptBuf::new(),
        script_type: ScriptType::P2WPKH,
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

// Test case to check if the fee is being rounded up correctly, and we expect the fee to be a positive integer
#[test]
fn test_fee_rounding_up() {
    let inputs = vec![utxo(100000)];
    let payments = vec![payment(50000)];

    let (fee, _) = estimate_fee(&inputs, &payments, true, ScriptType::P2WPKH, 5.0);

    assert!(fee > 0);
}

// Test case to check if the fee rate is being applied correctly, and we expect the fee to be approximately equal
// to the fee rate multiplied by the vbytes
#[test]
fn test_fee_rate_consistency() {
    let inputs = vec![utxo(100000)];
    let payments = vec![payment(50000)];

    let (fee, vbytes) = estimate_fee(&inputs, &payments, true, ScriptType::P2WPKH, 5.0);
    let rate = fee as f64 / vbytes as f64;

    assert!((rate - 5.0).abs() < 0.1);
}

// Test case to check if the vbytes estimation is consistent, and we expect the vbytes to be a positive integer
#[test]
fn test_vbytes_positive() {
    let inputs = vec![utxo(100000)];
    let payments = vec![payment(50000)];

    let (_, vbytes) = estimate_fee(&inputs, &payments, true, ScriptType::P2WPKH, 5.0);

    assert!(vbytes > 0);
}

// Test to check if the fee with change is greater than the fee without change since change increases the tx size
#[test]
fn test_fee_with_change_higher() {
    let inputs = vec![utxo(100000)];
    let payments = vec![payment(50000)];

    let (fee_change, _) = estimate_fee(&inputs, &payments, true, ScriptType::P2WPKH, 5.0);
    let (fee_no_change, _) = estimate_fee(&inputs, &payments, false, ScriptType::P2WPKH, 5.0);

    assert!(fee_change > fee_no_change);
}
