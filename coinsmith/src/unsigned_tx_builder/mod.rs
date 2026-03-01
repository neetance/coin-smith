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

pub fn build_unsigned_tx(
    payments: &[ValidatedPayment],
    coins: &CoinSelectionResult,
    change: &ValidatedChange,
    rbf: Option<bool>,
    locktime: Option<u32>,
    current_height: Option<u32>,
    fee_rate_sat_vb: f64,
) -> Result<PsbtResult, Box<dyn Error>> {
    let rbf_enabled = rbf.unwrap_or(false);

    let nlocktime = if let Some(locktime_val) = locktime {
        locktime_val
    } else if rbf_enabled {
        current_height.unwrap_or(0)
    } else {
        0
    };

    let sequence_value = if rbf_enabled {
        0xFFFFFFFD
    } else if locktime.is_some() {
        0xFFFFFFFE
    } else {
        0xFFFFFFFF
    };

    let lock_time = if nlocktime == 0 {
        LockTime::ZERO
    } else if nlocktime < 500_000_000 {
        LockTime::from_height(nlocktime)?
    } else {
        LockTime::from_time(nlocktime)?
    };

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

    let mut psbt = Psbt::from_unsigned_tx(unsigned_tx)?;

    for (idx, input) in coins.selected_coins.iter().enumerate() {
        psbt.inputs[idx].witness_utxo = Some(TxOut {
            value: Amount::from_sat(input.value_sats),
            script_pubkey: input.script_pubkey.clone(),
        });
    }

    let psbt_base64 = general_purpose::STANDARD.encode(psbt.serialize());

    let mut warnings = Vec::new();

    if coins.total_fee > 1_000_000 || fee_rate_sat_vb > 200.0 {
        warnings.push(Warning {
            code: "HIGH_FEE".to_string(),
        });
    }

    if !coins.change_included {
        warnings.push(Warning {
            code: "SEND_ALL".to_string(),
        });
    }

    if coins.change_included && coins.change_value < 546 {
        warnings.push(Warning {
            code: "DUST_CHANGE".to_string(),
        });
    }

    if sequence_value <= 0xFFFFFFFD {
        warnings.push(Warning {
            code: "RBF_SIGNALING".to_string(),
        });
    }

    let locktime_type = if nlocktime == 0 {
        "none"
    } else if nlocktime < 500_000_000 {
        "block_height"
    } else {
        "unix_timestamp"
    }
    .to_string();

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
    let fee_rate = coins.total_fee as f64 / (coins.vbytes as f64);

    Ok(PsbtResult {
        ok: true,
        network: "mainnet".to_string(),
        strategy: "greedy".to_string(),
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

fn script_type_str(script_type: ScriptType) -> String {
    match script_type {
        ScriptType::P2PKH => "p2pkh",
        ScriptType::P2SH => "p2sh",
        ScriptType::P2SH_P2WPKH => "p2sh-p2wpkh",
        ScriptType::P2TR => "p2tr",
        ScriptType::P2WPKH => "p2wpkh",
    }
    .to_string()
}
