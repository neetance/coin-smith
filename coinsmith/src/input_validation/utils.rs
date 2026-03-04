/*
 This module defines functions for validating raw utxos, payments, change and script
 These functions are of utility to the main validate_raw_fixture function
*/

use crate::input_validation::types::*;
use bitcoin::{ScriptBuf, Txid};
use std::str::FromStr;

// This function receives a list of raw utxos that are given in the raw fixture, checks
// for any malformities and returns a list of validated utxos in case of no error
pub fn validate_utxos(utxos: &[RawUtxo]) -> Result<Vec<ValidatedUtxo>, ValidationError> {
    // checking if inputs aren't empty
    if utxos.is_empty() {
        return Err(ValidationError::new(
            "EMPTY_UTXOS",
            "utxos array cannot be empty",
        ));
    }

    let mut validated_utxos = Vec::with_capacity(utxos.len());
    for utxo in utxos {
        if utxo.txid.len() != 64 {
            // checking for valid tx id length
            return Err(ValidationError::new(
                "INVALID_TXID",
                "txid must be 64 hex characters",
            ));
        }

        let txid = Txid::from_str(&utxo.txid)
            .map_err(|_| ValidationError::new("INVALID_TXID", "txid is not valid hex"))?;

        if utxo.value_sats == 0 {
            // checking if utxo value is positive
            return Err(ValidationError::new(
                "INVALID_VALUE",
                "UTXO value_sats must be greater than 0",
            ));
        }

        if utxo.script_pubkey_hex.is_empty() {
            // checking if the script pubkey hex string is not empty
            return Err(ValidationError::new(
                "INVALID_SCRIPT",
                "script_pubkey_hex cannot be empty",
            ));
        }

        let (script_pubkey, script_type) =
            validate_script(&utxo.script_pubkey_hex, &utxo.script_type)?; // validating the script type

        let address = utxo.address.clone();

        // finally adding the validated utxo in the list of validated utxos that will be returned
        validated_utxos.push(ValidatedUtxo {
            txid,
            vout: utxo.vout,
            value_sats: utxo.value_sats,
            script_pubkey: script_pubkey,
            script_type,
            address: address,
        });
    }

    Ok(validated_utxos)
}

// This function is for validating the raw payments list that we receive from the fixture input
pub fn validate_payments(
    payments: &[RawPayment],
) -> Result<Vec<ValidatedPayment>, ValidationError> {
    if payments.is_empty() {
        // payment list should have 1 or more payments
        return Err(ValidationError::new(
            "INVALID_PAYMENTS",
            "Payments cannot be empty",
        ));
    }

    let mut validated_payments = Vec::new();
    for payment in payments {
        if payment.value_sats == 0 {
            // value of payment can't be 0
            return Err(ValidationError::new(
                "INVALID_VALUE",
                "Payment value cannot be 0",
            ));
        }

        if payment.script_pubkey_hex.is_empty() {
            // checking if script pubkey hex is not an empty string
            return Err(ValidationError::new(
                "INVALID_SCRIPT",
                "script_pubkey_hex cannot be empty",
            ));
        }

        let (script_pubkey, script_type) =
            validate_script(&payment.script_pubkey_hex, &payment.script_type)?; // validating the script

        let address = payment.address.clone();

        // finally adding the validated payment to the validated payments list that will be returned at the end of the function
        validated_payments.push(ValidatedPayment {
            address: address,
            script_pubkey_hex: script_pubkey,
            script_type: script_type,
            value_sats: payment.value_sats,
        });
    }

    Ok(validated_payments)
}

// This function is for validating the change part of the fixture input
pub fn validate_change(change: &RawChange) -> Result<ValidatedChange, ValidationError> {
    if change.script_pubkey_hex.is_empty() {
        // checking if script pubkey hex is not an empty string
        return Err(ValidationError::new(
            "INVALID_SCRIPT",
            "script_pubkey_hex cannot be empty",
        ));
    }

    let (script_pubkey, script_type) =
        validate_script(&change.script_pubkey_hex, &change.script_type)?; // validating the script
    let address = change.address.clone();

    // finally returning the validated change
    Ok(ValidatedChange {
        address,
        script_pubkey_hex: script_pubkey,
        script_type,
    })
}

// This function is for validating the script pubkey hex and script type, and checking if they match each other
fn validate_script(
    script_pubkey_hex: &str,
    script_type: &str,
) -> Result<(ScriptBuf, ScriptType), ValidationError> {
    let script_bytes = hex::decode(script_pubkey_hex).map_err(|_| {
        ValidationError::new("INVALID_SCRIPT", "script_pubkey_hex is not valid hex")
    })?; // checking if the script pubkey hex string is a valid hex string and converting it to bytes

    let script_pubkey = ScriptBuf::from_bytes(script_bytes); // converting the script pubkey bytes to a ScriptBuf type
    let script_type = match script_type {
        // matching the string script type to the ScriptType enum and checking if it's a supported script type
        "p2wpkh" => ScriptType::P2WPKH,
        "p2pkh" => ScriptType::P2PKH,
        "p2tr" => ScriptType::P2TR,
        "p2sh-p2wpkh" => ScriptType::P2SH_P2WPKH,
        _ => {
            return Err(ValidationError::new(
                "UNSUPPORTED_SCRIPT_TYPE",
                "Unsupported script_type",
            ));
        }
    };

    // checking if the script pubkey matches the declared script type
    let script_matches = match script_type {
        ScriptType::P2WPKH => script_pubkey.is_p2wpkh(),
        ScriptType::P2PKH => script_pubkey.is_p2pkh(),
        ScriptType::P2TR => script_pubkey.is_p2tr(),
        ScriptType::P2SH_P2WPKH => script_pubkey.is_p2sh(),
    };

    // if the script pubkey doesn't match the declared script type, we raise an error
    if !script_matches {
        return Err(ValidationError::new(
            "SCRIPT_MISMATCH",
            "script_pubkey does not match declared script_type",
        ));
    }

    // finally returning the validated script pubkey and script type
    Ok((script_pubkey, script_type))
}
