/*
 This module implements the logic for utxo consolidation. What utxo consolidatoin is:
 It is the concept of consolidating small utxos into larger ones, in order to save on fees in the long term. This is because
 each input in a transaction adds to the size of the transaction, and therefore increases the fee. By consolidating small utxos
 into larger ones, we can reduce the number of inputs in future transactions, and therefore save on fees in the long term.
*/

use crate::coin_selection::{CoinSelectionError, CoinSelectionResult, fee_estimator::estimate_fee};
use crate::input_validation::types::{ScriptType, ValidatedPayment, ValidatedUtxo};

// this is the fee rate threshold below which we consider consolidating utxos, since at low fee rates, the long term savings
// from consolidation are more likely to outweigh the short term cost of consolidation.
const LOW_FEE_THRESHOLD: f64 = 5.0;

// this is the fee rate we use to estimate the long term savings from consolidation, since at higher fee rates, the savings
// from consolidation are more significant.
const LONG_TERM_FEE_RATE: f64 = 20.0;

// this is the value threshold below which we consider a utxo for consolidation, since very large utxos are less likely to
// be beneficial to consolidate, and we want to focus on consolidating smaller utxos that are more likely to be beneficial to consolidate.
const UTXO_VALUE_THRESHOLD: u64 = 10000;

// this is the minimum change value we would consider including in the transaction. If the change is below this threshold,
// we will not include it and instead add it to the fee.
const DUST_THRESHOLD: u64 = 546;

// This is the main function for utxo consolidation
pub fn consolidate_utxos(
    all_utxos: &[ValidatedUtxo],
    payments: &[ValidatedPayment],
    coins: CoinSelectionResult,
    change_script_type: ScriptType,
    fee_rate_sat_vb: f64,
    max_inputs: u32,
) -> Result<CoinSelectionResult, CoinSelectionError> {
    // if the fee rate is above the low fee threshold, we skip consolidation and return the original coin selection result,
    // since at higher fee rates, the short term cost of consolidation is more likely to outweigh the long term savings.
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

    // here we calculate the remaining utxos that are not in the current list of selected coins
    let mut remaining: Vec<ValidatedUtxo> = all_utxos
        .iter()
        .filter(|u| {
            !selected_coins
                .iter()
                .any(|s| s.txid == u.txid && s.vout == u.vout)
        })
        .cloned()
        .collect();

    // sorting remaining utxos by value in ascending order, so that we consider consolidating smaller utxos first, since
    // they are more likely to be beneficial to consolidate.
    remaining.sort_by(|a, b| a.value_sats.cmp(&b.value_sats));

    // looping over each remaining utxo
    for utxo in remaining {
        // if we have already selected the maximum number of inputs allowed by the policy, we stop and break out of the loop,
        // since we don't want to exceed the maximum number of inputs allowed by the policy.
        if selected_coins.len() as u32 >= max_inputs {
            break;
        }

        // if the utxo value is above the utxo value threshold, we skip it and continue to the next utxo, since very large
        // utxos are less likely to be beneficial to consolidate.
        if utxo.value_sats > UTXO_VALUE_THRESHOLD {
            continue;
        }

        // we create an intermediate selection that includes the current selected coins plus the utxo we are considering for consolidation
        let mut intermediate_selection = selected_coins.clone();
        intermediate_selection.push(utxo.clone());

        // we calcualte the new fee and the new size in vbytes after adding this utxo to the selection
        let (new_fee, new_vbytes) = estimate_fee(
            &intermediate_selection,
            payments,
            true,
            change_script_type,
            fee_rate_sat_vb,
        );

        // we calculate the new required amount, which is the sum of the payment amount and the new fee.
        // this represents the total amount we need to cover with our selected inputs if we include change after adding the new utxo.
        let new_required = payment_sum
            .checked_add(new_fee)
            .ok_or_else(|| CoinSelectionError::new("AMOUNT_OVERFLOW", "Required overflow"))?;

        // we calculate the new total input value after adding the new utxo to the selection.
        let new_total_input = total_input_value
            .checked_add(utxo.value_sats)
            .ok_or_else(|| CoinSelectionError::new("AMOUNT_OVERFLOW", "Input overflow"))?;

        // if the new total input value is not sufficient to cover the new required amount, we skip this utxo and continue to
        // the next one, since it would not be beneficial to consolidate this utxo if it does not help us cover the required amount.
        if new_total_input < new_required {
            continue;
        }

        // we calculate the new change value after adding the new utxo to the selection, which is the new total input
        // value minus the new required amount.
        // if the new change value is below the dust threshold, we skip this utxo and continue to the next one, since it
        // would not be beneficial to consolidate this utxo if the change would be dust.
        let new_change = new_total_input - new_required;
        if new_change < DUST_THRESHOLD {
            continue;
        }

        // we calculate the fee increase from adding this utxo to the selection, which is the new fee minus the old fee.
        // we also calculate the future savings from consolidating this utxo, which is the estimated fee to spend this utxo
        // in the future, which is the input vbytes of this utxo multiplied by the long term fee rate.
        let fee_increase = new_fee.saturating_sub(total_fee);
        let input_vbytes = get_input_vbytes(utxo.script_type);
        let future_savings = (input_vbytes as f64 * LONG_TERM_FEE_RATE) as u64;

        // if the future savings from consolidating this utxo are greater than the fee increase from adding this utxo to the
        // selection, we update our selected coins and the corresponding total input value, total fee, change value, and vbytes
        // to reflect the addition of this utxo to the selection, since it would be beneficial to consolidate this utxo if the
        // long term savings outweigh the short term cost.
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

    // finally, we return the new coin selection result after consolidation
    Ok(CoinSelectionResult {
        selected_coins,
        total_input_value,
        total_fee,
        change_included: true,
        change_value,
        vbytes,
    })
}

// helper function to get the input vbytes for a given script type
fn get_input_vbytes(script_type: ScriptType) -> usize {
    let vbytes = match script_type {
        ScriptType::P2PKH => 148,
        ScriptType::P2WPKH => 68,
        ScriptType::P2SH_P2WPKH => 91,
        ScriptType::P2TR => 58,
    };

    vbytes
}
