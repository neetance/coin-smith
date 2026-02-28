use crate::input_validation::types::*;
use bitcoin::{ScriptBuf, Txid};
use std::str::FromStr;

pub fn validate_utxos(utxos: &[RawUtxo]) -> Result<Vec<ValidatedUtxo>, ValidationError> {
    if utxos.is_empty() {
        return Err(ValidationError::new(
            "EMPTY_UTXOS",
            "utxos array cannot be empty",
        ));
    }

    let mut validated_utxos = Vec::with_capacity(utxos.len());
    for utxo in utxos {
        if utxo.txid.len() != 64 {
            return Err(ValidationError::new(
                "INVALID_TXID",
                "txid must be 64 hex characters",
            ));
        }

        let txid = Txid::from_str(&utxo.txid)
            .map_err(|_| ValidationError::new("INVALID_TXID", "txid is not valid hex"))?;

        if utxo.value_sats == 0 {
            return Err(ValidationError::new(
                "INVALID_VALUE",
                "UTXO value_sats must be greater than 0",
            ));
        }

        if utxo.script_pubkey_hex.is_empty() {
            return Err(ValidationError::new(
                "INVALID_SCRIPT",
                "script_pubkey_hex cannot be empty",
            ));
        }

        let (script_pubkey, script_type) =
            validate_script(&utxo.script_pubkey_hex, &utxo.script_type)?;

        let address = utxo.address.clone();
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

pub fn validate_payments(
    payments: &[RawPayment],
) -> Result<Vec<ValidatedPayment>, ValidationError> {
    if payments.is_empty() {
        return Err(ValidationError::new(
            "INVALID_PAYMENTS",
            "Payments cannot be empty",
        ));
    }

    let mut validated_payments = Vec::new();
    for payment in payments {
        if payment.value_sats == 0 {
            return Err(ValidationError::new(
                "INVALID_VALUE",
                "Payment value cannot be 0",
            ));
        }

        if payment.script_pubkey_hex.is_empty() {
            return Err(ValidationError::new(
                "INVALID_SCRIPT",
                "script_pubkey_hex cannot be empty",
            ));
        }

        let (script_pubkey, script_type) =
            validate_script(&payment.script_pubkey_hex, &payment.script_type)?;

        let address = payment.address.clone();
        validated_payments.push(ValidatedPayment {
            address: address,
            script_pubkey_hex: script_pubkey,
            script_type: script_type,
            value_sats: payment.value_sats,
        });
    }

    Ok(validated_payments)
}

pub fn validate_change(change: &RawChange) -> Result<ValidatedChange, ValidationError> {
    if change.script_pubkey_hex.is_empty() {
        return Err(ValidationError::new(
            "INVALID_SCRIPT",
            "script_pubkey_hex cannot be empty",
        ));
    }

    let (script_pubkey, script_type) =
        validate_script(&change.script_pubkey_hex, &change.script_type)?;
    let address = change.address.clone();

    Ok(ValidatedChange {
        address,
        script_pubkey_hex: script_pubkey,
        script_type,
    })
}

fn validate_script(
    script_pubkey_hex: &str,
    script_type: &str,
) -> Result<(ScriptBuf, ScriptType), ValidationError> {
    let script_bytes = hex::decode(script_pubkey_hex).map_err(|_| {
        ValidationError::new("INVALID_SCRIPT", "script_pubkey_hex is not valid hex")
    })?;

    let script_pubkey = ScriptBuf::from_bytes(script_bytes);
    let script_type = match script_type {
        "p2wpkh" => ScriptType::P2WPKH,
        "p2pkh" => ScriptType::P2PKH,
        "p2sh" => ScriptType::P2SH,
        "p2tr" => ScriptType::P2TR,
        _ => {
            return Err(ValidationError::new(
                "UNSUPPORTED_SCRIPT_TYPE",
                "Unsupported script_type",
            ));
        }
    };

    let script_matches = match script_type {
        ScriptType::P2WPKH => script_pubkey.is_p2wpkh(),
        ScriptType::P2PKH => script_pubkey.is_p2pkh(),
        ScriptType::P2SH => script_pubkey.is_p2sh(),
        ScriptType::P2TR => script_pubkey.is_p2tr(),
    };

    if !script_matches {
        return Err(ValidationError::new(
            "SCRIPT_MISMATCH",
            "script_pubkey does not match declared script_type",
        ));
    }

    Ok((script_pubkey, script_type))
}
