// This module defines the various types which we will be using througout the project

use bitcoin;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

// Raw fixture that we will be receiving as inputs. Will be checked if it is malformed before going to
// coin selection
#[derive(Debug, Deserialize)]
pub struct RawFixture {
    pub network: String,
    pub utxos: Vec<RawUtxo>,
    pub payments: Vec<RawPayment>,
    pub change: RawChange,
    pub fee_rate_sat_vb: f64,
    pub rbf: Option<bool>,
    pub locktime: Option<u32>,
    pub current_height: Option<u32>,
    pub policy: Option<RawPolicy>,
}

// Raw version of the utxo received from raw fixture
#[derive(Debug, Deserialize)]
pub struct RawUtxo {
    pub txid: String,
    pub vout: u32,
    pub value_sats: u64,
    pub script_pubkey_hex: String,
    pub script_type: String,
    pub address: Option<String>,
}

// Raw payment from the raw fixture input
#[derive(Debug, Deserialize)]
pub struct RawPayment {
    pub address: Option<String>,
    pub script_pubkey_hex: String,
    pub script_type: String,
    pub value_sats: u64,
}

// Raw change from the raw fixture
#[derive(Debug, Deserialize)]
pub struct RawChange {
    pub address: Option<String>,
    pub script_pubkey_hex: String,
    pub script_type: String,
}

// raw version of the policy from the raw fixture
#[derive(Debug, Deserialize)]
pub struct RawPolicy {
    pub max_inputs: Option<u32>,
}

// This is the structure for the validated fixture, which will be constructed after performing all malformity checks
pub struct ValidatedFixture {
    pub network: String,
    pub utxos: Vec<ValidatedUtxo>,
    pub payments: Vec<ValidatedPayment>,
    pub change: ValidatedChange,
    pub fee_rate_sat_vb: f64,
    pub rbf: Option<bool>,
    pub locktime: Option<u32>,
    pub current_height: Option<u32>,
    pub policy: Option<ValidatedPolicy>,
}

// Validated utxo, constructed after performning all checks on raw version of the utxos
#[derive(Serialize)]
pub struct ValidatedUtxo {
    pub txid: bitcoin::Txid,
    pub vout: u32,
    pub value_sats: u64,
    pub script_pubkey: bitcoin::ScriptBuf,
    pub script_type: ScriptType,
    pub address: Option<String>,
}

impl Clone for ValidatedUtxo {
    fn clone(&self) -> Self {
        return Self {
            txid: self.txid,
            vout: self.vout,
            value_sats: self.value_sats,
            script_pubkey: self.script_pubkey.clone(),
            script_type: self.script_type,
            address: self.address.clone(),
        };
    }
}

// Validated version of the payment, after validating the raw payment version
pub struct ValidatedPayment {
    pub address: Option<String>,
    pub script_pubkey_hex: bitcoin::ScriptBuf,
    pub script_type: ScriptType,
    pub value_sats: u64,
}

// validated version of the raw change struct
pub struct ValidatedChange {
    pub address: Option<String>,
    pub script_pubkey_hex: bitcoin::ScriptBuf,
    pub script_type: ScriptType,
}

// validated version of the policy
pub struct ValidatedPolicy {
    pub max_inputs: Option<u32>,
}

impl ValidatedPolicy {
    pub fn new() -> Self {
        Self { max_inputs: None }
    }

    pub fn add_max_inputs(&mut self, num_inputs: u32) {
        self.max_inputs = Some(num_inputs);
    }
}

// This is the definition of the error we will be raising if there is some malformity in the inputs,
// with error code and message
#[derive(Debug)]
pub struct ValidationError {
    pub code: String,
    pub message: String,
}

impl ValidationError {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
        }
    }
}

// This is the enum for the standard script types(p2pkh, p2wpkh, p2sh-p2wpkh, p2tr)
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ScriptType {
    #[serde(rename = "p2pkh")]
    P2PKH,
    #[serde(rename = "p2wpkh")]
    P2WPKH,
    #[serde(rename = "p2tr")]
    P2TR,
    #[serde(rename = "p2sh-p2wpkh")]
    P2SH_P2WPKH,
}
