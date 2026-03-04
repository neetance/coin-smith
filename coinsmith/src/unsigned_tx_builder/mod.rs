/*
 This module is responsible for building the final unsigned transaction based on the selected coins, payments, and
 change output.
*/

use serde::Serialize;
use std::error::Error;

use crate::coin_selection::CoinSelectionResult;
use crate::input_validation::types::{
    ScriptType, ValidatedChange, ValidatedPayment, ValidatedUtxo,
};
use base64::{Engine as _, engine::general_purpose};
use bitcoin::absolute::LockTime;
use bitcoin::transaction::Version;
use bitcoin::{Amount, Transaction, TxIn, TxOut, psbt::Psbt};
use bitcoin::{OutPoint, ScriptBuf, Sequence, Witness};

#[derive(Serialize)]
pub struct Output {
    pub n: usize,
    pub value_sats: u64,
    pub script_pubkey_hex: String,
    pub script_type: String,
    pub address: Option<String>,
    pub is_change: bool,
}

#[derive(Serialize)]
pub struct Warning {
    pub code: String,
}

// This struct represents the final psbt result which will be returned by this module and will be displayed in the json format
#[derive(Serialize)]
pub struct PsbtResult {
    pub ok: bool,
    pub network: String,
    pub strategy: String,
    pub selected_inputs: Vec<ValidatedUtxo>,
    pub outputs: Vec<Output>,
    pub change_index: Option<usize>,
    pub fee_sats: u64,
    pub fee_rate_sat_vb: f64,
    pub vbytes: usize,
    pub rbf_signaling: bool,
    pub locktime: u32,
    pub locktime_type: String,
    pub psbt_base64: String,
    pub warnings: Vec<Warning>,
}

// This is the struct for the error which we will be raising while getting an error in this module
#[derive(Debug)]
pub struct TxBuilderError {
    pub code: String,
    pub message: String,
}

impl TxBuilderError {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
        }
    }
}

// This is the main function to build an unsigned tx, which returns the psbt struct in case of no error
pub fn build_unsigned_tx(
    strategy: String,
    payments: &[ValidatedPayment],
    coins: &CoinSelectionResult,
    change: &ValidatedChange,
    rbf: Option<bool>,
    locktime: Option<u32>,
    current_height: Option<u32>,
    fee_rate_sat_vb: f64,
) -> Result<PsbtResult, Box<dyn Error>> {
    // we determine the sequence value and locktime for the transaction based on the rbf and locktime parameters.
    let rbf_enabled = rbf.unwrap_or(false);

    let nlocktime = if let Some(locktime_val) = locktime {
        locktime_val
    } else if rbf_enabled {
        current_height.unwrap_or(0)
    } else {
        0
    };

    // If rbf is enabled, we set the sequence value to 0xFFFFFFFD to signal that this transaction can be replaced.
    // If locktime is provided, we set the sequence value to 0xFFFFFFFE to signal that this transaction has a locktime.
    // If neither is provided, we set the sequence value to 0xFFFFFFFF to signal that this transaction is final and cannot
    // be replaced or have a locktime.
    let sequence_value = if rbf_enabled {
        0xFFFFFFFD
    } else if locktime.is_some() {
        0xFFFFFFFE
    } else {
        0xFFFFFFFF
    };

    // we determine the locktime for the transaction based on the nlocktime value, which is derived from the locktime
    // parameter and the current block height if rbf is enabled. If nlocktime is 0, we set the locktime to zero.
    // If nlocktime is less than 500 million, we interpret it as a block height and set the locktime accordingly.
    // If nlocktime is greater than 500 million, we interpret it as a unix timestamp and set the locktime accordingly.
    let lock_time = if nlocktime == 0 {
        LockTime::ZERO
    } else if nlocktime < 500_000_000 {
        LockTime::from_height(nlocktime)?
    } else {
        LockTime::from_time(nlocktime)?
    };

    // we construct the unsigned transaction using the selected coins as inputs, the payments as outputs, and the change output if included.
    let mut tx_inputs = Vec::new();
    for input in &coins.selected_coins {
        tx_inputs.push(TxIn {
            previous_output: OutPoint {
                txid: input.txid,
                vout: input.vout,
            },
            script_sig: ScriptBuf::new(),
            sequence: Sequence(sequence_value),
            witness: Witness::new(),
        });
    }

    let mut tx_outputs = Vec::new();

    for payment in payments {
        tx_outputs.push(TxOut {
            value: Amount::from_sat(payment.value_sats),
            script_pubkey: payment.script_pubkey_hex.clone(),
        });
    }

    let change_index = if coins.change_included {
        let idx = tx_outputs.len();
        tx_outputs.push(TxOut {
            value: Amount::from_sat(coins.change_value),
            script_pubkey: change.script_pubkey_hex.clone(),
        });
        Some(idx)
    } else {
        None
    };

    let unsigned_tx = Transaction {
        version: Version(2),
        lock_time,
        input: tx_inputs,
        output: tx_outputs,
    };

    // we create a psbt from the unsigned transaction, and we populate the witness_utxo field for each input in the psbt
    // with the corresponding UTXO information from the selected coins, which will be used by the signer to know the
    // details of the inputs being spent.
    let mut psbt = Psbt::from_unsigned_tx(unsigned_tx)?;
    for (idx, input) in coins.selected_coins.iter().enumerate() {
        psbt.inputs[idx].witness_utxo = Some(TxOut {
            value: Amount::from_sat(input.value_sats),
            script_pubkey: input.script_pubkey.clone(),
        });
    }

    // we encode the psbt as a base64 string to be included in the final result, which can be easily transmitted and
    // decoded by the client.
    let psbt_base64 = general_purpose::STANDARD.encode(psbt.serialize());

    // we prepare a list of warnings to include in the final result based on certain conditions
    let mut warnings = Vec::new();

    // if the total fee is above a certain threshold or the fee rate is above a certain threshold, we include a warning about
    // high fees in the result.
    if coins.total_fee > 1_000_000 || fee_rate_sat_vb > 200.0 {
        warnings.push(Warning {
            code: "HIGH_FEE".to_string(),
        });
    }

    // if the total fee is above the payment amount, we include a warning about the fee being higher than the payment amount
    // in the result.
    if !coins.change_included {
        warnings.push(Warning {
            code: "SEND_ALL".to_string(),
        });
    }

    // if the change value is below the dust threshold, we include a warning about dust change in the result, since this change
    // output would not be economical to spend in the future and might be considered dust by the network.
    if coins.change_included && coins.change_value < 546 {
        warnings.push(Warning {
            code: "DUST_CHANGE".to_string(),
        });
    }

    // if the sequence value is set to signal rbf, we include a warning about rbf signaling in the result, since this transaction
    // can be replaced and the user should be aware of this when signing and broadcasting the transaction
    if sequence_value <= 0xFFFFFFFD {
        warnings.push(Warning {
            code: "RBF_SIGNALING".to_string(),
        });
    }

    // we determine the locktime type based on the nlocktime value
    let locktime_type = if nlocktime == 0 {
        // none if 0
        "none"
    } else if nlocktime < 500_000_000 {
        // block height if less than 500 million
        "block_height"
    } else {
        // unix timestamp if greater than 500 million
        "unix_timestamp"
    }
    .to_string();

    // creating the final outputs report for the final psbt result
    let mut outputs_report = Vec::new();
    for (idx, payment) in payments.iter().enumerate() {
        outputs_report.push(Output {
            n: idx,
            value_sats: payment.value_sats,
            script_pubkey_hex: payment.script_pubkey_hex.to_hex_string(),
            script_type: script_type_str(payment.script_type),
            address: payment.address.clone(),
            is_change: false,
        });
    }

    // adding change output if change is included
    if coins.change_included {
        outputs_report.push(Output {
            n: change_index.unwrap(),
            value_sats: coins.change_value,
            script_pubkey_hex: change.script_pubkey_hex.to_hex_string(),
            script_type: script_type_str(change.script_type),
            address: change.address.clone(),
            is_change: true,
        });
    }

    // we calculate the fee rate in satoshis per vbyte for the final result by dividing the total fee by the total vbytes of the transaction,
    // and we round it to 2 decimal places for better readability in the result.
    let mut fee_rate = coins.total_fee as f64 / (coins.vbytes as f64);
    fee_rate = (fee_rate * 100.0).round() / 100.0;

    // finally, we return the psbt result struct which will be displayed in the json format as the final result
    Ok(PsbtResult {
        ok: true,
        network: "mainnet".to_string(),
        strategy,
        selected_inputs: coins.selected_coins.clone(),
        outputs: outputs_report,
        change_index,
        fee_sats: coins.total_fee,
        fee_rate_sat_vb: fee_rate,
        vbytes: coins.vbytes,
        rbf_signaling: sequence_value <= 0xFFFFFFFD,
        locktime: nlocktime,
        locktime_type,
        psbt_base64,
        warnings,
    })
}

// helper function to convert script type enum to string
fn script_type_str(script_type: ScriptType) -> String {
    match script_type {
        ScriptType::P2PKH => "p2pkh",
        ScriptType::P2SH_P2WPKH => "p2sh-p2wpkh",
        ScriptType::P2TR => "p2tr",
        ScriptType::P2WPKH => "p2wpkh",
    }
    .to_string()
}
