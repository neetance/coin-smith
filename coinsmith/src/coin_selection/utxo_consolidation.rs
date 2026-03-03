use crate::coin_selection::{CoinSelectionError, CoinSelectionResult, fee_estimator::estimate_fee};
use crate::input_validation::types::{ScriptType, ValidatedPayment, ValidatedUtxo};

const LOW_FEE_THRESHOLD: f64 = 5.0;
const LONG_TERM_FEE_RATE: f64 = 20.0;
const UTXO_VALUE_THRESHOLD: u64 = 10000;
const DUST_THRESHOLD: u64 = 546;

pub fn consolidate_utxos(
    all_utxos: &[ValidatedUtxo],
    payments: &[ValidatedPayment],
    coins: CoinSelectionResult,
    change_script_type: ScriptType,
    fee_rate_sat_vb: f64,
    max_inputs: u32,
) -> Result<CoinSelectionResult, CoinSelectionError> {
    if fee_rate_sat_vb >= LOW_FEE_THRESHOLD {
        return Ok(coins);
    }

    let mut selected_coins = coins.selected_coins.clone();
    let mut total_input_value = coins.total_input_value;
    let mut total_fee = coins.total_fee;
    let mut change_value = coins.change_value;
    let mut vbytes = coins.vbytes;

    let mut payment_sum: u64 = 0;
    for p in payments {
        payment_sum = payment_sum
            .checked_add(p.value_sats)
            .ok_or_else(|| CoinSelectionError::new("AMOUNT_OVERFLOW", "Payment overflow"))?;
    }

    let mut remaining: Vec<ValidatedUtxo> = all_utxos
        .iter()
        .filter(|u| {
            !selected_coins
                .iter()
                .any(|s| s.txid == u.txid && s.vout == u.vout)
        })
        .cloned()
        .collect();
    remaining.sort_by(|a, b| a.value_sats.cmp(&b.value_sats));

    for utxo in remaining {
        if selected_coins.len() as u32 >= max_inputs {
            break;
        }
        if utxo.value_sats > UTXO_VALUE_THRESHOLD {
            continue;
        }

        let mut intermediate_selection = selected_coins.clone();
        intermediate_selection.push(utxo.clone());

        let (new_fee, new_vbytes) = estimate_fee(
            &intermediate_selection,
            payments,
            true,
            change_script_type,
            fee_rate_sat_vb,
        );

        let new_required = payment_sum
            .checked_add(new_fee)
            .ok_or_else(|| CoinSelectionError::new("AMOUNT_OVERFLOW", "Required overflow"))?;

        let new_total_input = total_input_value
            .checked_add(utxo.value_sats)
            .ok_or_else(|| CoinSelectionError::new("AMOUNT_OVERFLOW", "Input overflow"))?;

        if new_total_input < new_required {
            continue;
        }

        let new_change = new_total_input - new_required;
        if new_change < DUST_THRESHOLD {
            continue;
        }

        let fee_increase = new_fee.saturating_sub(total_fee);
        let input_vbytes = get_input_vbytes(utxo.script_type);
        let future_savings = (input_vbytes as f64 * LONG_TERM_FEE_RATE) as u64;

        if future_savings > fee_increase {
            selected_coins = intermediate_selection;
            total_input_value = new_total_input;
            total_fee = new_fee;
            change_value = new_change;
            vbytes = new_vbytes;
        } else {
            break;
        }
    }

    Ok(CoinSelectionResult {
        selected_coins,
        total_input_value,
        total_fee,
        change_included: true,
        change_value,
        vbytes,
    })
}

fn get_input_vbytes(script_type: ScriptType) -> usize {
    let vbytes = match script_type {
        ScriptType::P2PKH => 148,
        ScriptType::P2WPKH => 68,
        ScriptType::P2SH_P2WPKH => 91,
        ScriptType::P2TR => 58,
    };

    vbytes
}
