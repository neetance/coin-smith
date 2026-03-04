/*
 This module contains the fee estimation logic, which is used across different coin selection strategies to estimate
 the fee for a given selection of inputs and outputs.
*/

use crate::input_validation::types::{ScriptType, ValidatedPayment, ValidatedUtxo};

// The main function for calculating the fess for a given set of inputs and outputs.
pub fn estimate_fee(
    inputs: &[ValidatedUtxo],
    payments: &[ValidatedPayment],
    includes_change: bool,
    change_script_type: ScriptType,
    fee_rate_sat_vb: f64,
) -> (u64, usize) {
    let fee_rate_millisat_vb = (fee_rate_sat_vb * 1000.0).round() as u64;

    // we estimate the size of the tx
    let size_vb = estimate_size(inputs, payments, includes_change, change_script_type);
    let total_fee_sats = (fee_rate_millisat_vb * (size_vb as u64) + 999) / 1000; // rounding up the fee to the nearest satoshi

    (total_fee_sats, size_vb) // finally return the total fee in satoshis and the estimated size in vbytes
}

// Function for estimating the size of the transaction based on the number of inputs and outputs, and whether change is
// included or not. We also take into account the script types of the inputs and outputs, since they affect the size of
// the transaction.
fn estimate_size(
    inputs: &[ValidatedUtxo],
    payments: &[ValidatedPayment],
    includes_change: bool,
    change_script_type: ScriptType,
) -> usize {
    let mut weight = 0;
    weight += 4 * 4; // this is for the version, since version is of 4 bytes, and we multiply the weight of non-segwit part by 4
    weight += varint_size(inputs.len() as u64) * 4;

    let mut has_witness = false;
    for utxo in inputs {
        weight += calculate_input_base_size(utxo) * 4;
        if is_segwit(utxo) {
            has_witness = true;
            match utxo.script_type {
                ScriptType::P2WPKH | ScriptType::P2SH_P2WPKH => {
                    weight += 108;
                }
                ScriptType::P2TR => {
                    weight += 66;
                }
                _ => {}
            }
        }
    }
    if has_witness {
        weight += 2; // since marker flag for witness is of 2 bytes
    }

    let output_count = if includes_change {
        payments.len() + 1
    } else {
        payments.len()
    };

    weight += varint_size(output_count as u64) * 4;
    for payment in payments {
        weight += calculate_output_base_size(payment.script_type) * 4;
    }
    if includes_change {
        weight += calculate_output_base_size(change_script_type) * 4;
    }

    weight += 4 * 4; // for locktime since locktime is of 4 bytes

    let vbytes = (weight + 3) / 4;
    vbytes
}

fn varint_size(num_inputs: u64) -> usize {
    if num_inputs < 253 {
        return 1;
    } else if num_inputs <= 0xffff {
        return 3;
    } else if num_inputs <= 0xffffffff {
        return 5;
    } else {
        return 9;
    }
}

fn calculate_input_base_size(input: &ValidatedUtxo) -> usize {
    let mut size = 0;
    size += 32; // for txid since it is of 32 bytes
    size += 4; // for vout since it is of 4 bytes
    size += 1; // for scriptSig since it is of 1 byte

    match input.script_type {
        ScriptType::P2PKH => {
            size += 107;
        } // scriptSig size ~ 107 bytes for p2pkh
        ScriptType::P2SH_P2WPKH => {
            size += 22;
        } // p2sh contains redeem script of 22 bytes
        _ => {} // rest all are segwit script types, will be handled in the witness section
    }

    size += 4; // for sequence
    return size;
}

fn calculate_output_base_size(script_type: ScriptType) -> usize {
    let mut size = 0;
    size += 8; // for value, since the value is of 8 bytes
    size += 1; // for script lengh

    match script_type {
        ScriptType::P2PKH => {
            size += 25;
        } // p2pkh script length is of 25 bytes
        ScriptType::P2WPKH => {
            size += 22;
        } // p2wpkh script length is of 22 bytes
        ScriptType::P2TR => {
            size += 34;
        } // p2tr script length is of 34 bytes
        _ => size += 23, // p2sh script length is of 23 bytes
    }

    return size;
}

fn is_segwit(input: &ValidatedUtxo) -> bool {
    let is_segwit = match input.script_type {
        ScriptType::P2WPKH | ScriptType::P2TR | ScriptType::P2SH_P2WPKH => true,
        _ => false,
    };

    return is_segwit;
}
