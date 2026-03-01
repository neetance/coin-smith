use bitcoin;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Deserialize)]
pub struct RawUtxo {
    pub txid: String,
    pub vout: u32,
    pub value_sats: u64,
    pub script_pubkey_hex: String,
    pub script_type: String,
    pub address: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RawPayment {
    pub address: Option<String>,
    pub script_pubkey_hex: String,
    pub script_type: String,
    pub value_sats: u64,
}

#[derive(Debug, Deserialize)]
pub struct RawChange {
    pub address: Option<String>,
    pub script_pubkey_hex: String,
    pub script_type: String,
}

#[derive(Debug, Deserialize)]
pub struct RawPolicy {
    pub max_inputs: Option<u32>,
}

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

pub struct ValidatedPayment {
    pub address: Option<String>,
    pub script_pubkey_hex: bitcoin::ScriptBuf,
    pub script_type: ScriptType,
    pub value_sats: u64,
}

pub struct ValidatedChange {
    pub address: Option<String>,
    pub script_pubkey_hex: bitcoin::ScriptBuf,
    pub script_type: ScriptType,
}

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

#[derive(Debug, Clone, Copy, Serialize)]
pub enum ScriptType {
    P2WPKH,
    P2PKH,
    P2SH,
    P2TR,
    P2SH_P2WPKH,
}
