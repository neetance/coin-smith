/*
 Here we define the stochastic knapsack coin selection strategy, which is a randomized approach to finding a good selection
 of UTXOs. The algorithm works by shuffling the list of UTXOs and then iterating through them, adding them to the selection
 until we have enough input value to cover the payment amount plus fees. We repeat this process for a fixed number of iterations
 keeping track of the best solution found in terms of minimizing fees and change.
*/

use crate::coin_selection::fee_estimator::estimate_fee;
use crate::coin_selection::{CoinSelectionError, CoinSelectionResult};
use crate::input_validation::types::*;
use rand::seq::SliceRandom;
use rand::thread_rng;

const KNAPSACK_ITERATIONS: usize = 2000; // Capping the iterations to prevent long runtimes in large UTXO sets
const DUST_THRESHOLD: u64 = 546; // The minimum change value we would consider including in the transaction

pub fn select_coins_stochastic_knapsack(
    utxos: &[ValidatedUtxo],
    payments: &[ValidatedPayment],
    change: &ValidatedChange,
    fee_rate_sat_vb: f64,
    max_inputs: u32,
) -> Result<CoinSelectionResult, CoinSelectionError> {
    // First, we compute the total payment amount, which is the sum of all payment values. This is the target amount
    // we need to cover with our selected inputs plus fees.
    let mut payment_sum: u64 = 0;
    for p in payments {
        payment_sum = payment_sum
            .checked_add(p.value_sats)
            .ok_or_else(|| CoinSelectionError::new("AMOUNT_OVERFLOW", "Payment overflow"))?;
    }

    let mut best: Option<CoinSelectionResult> = None;
    let mut rng = thread_rng();

    // We perform a fixed number of iterations, where in each iteration we shuffle the list of UTXOs and try to
    // find a valid selection.
    for _ in 0..KNAPSACK_ITERATIONS {
        let mut shuffled = utxos.to_vec();
        shuffled.shuffle(&mut rng); // Randomly shuffle the UTXOs to explore different combinations in each iteration

        let mut selected = Vec::new();
        let mut total_input: u64 = 0;

        // For each shuffled UTXO, we add it to the selection and check if we have enough total input value to cover
        // the payment plus fees.
        for utxo in shuffled.iter() {
            // If we have already selected the maximum number of inputs allowed by the policy, we stop and move to the
            // next iteration.
            if selected.len() as u32 >= max_inputs {
                break;
            }

            // adding the value of the current UTXO to the total input sum, and we add the corresponding UTXO to the list
            // of selected coins.
            total_input = total_input
                .checked_add(utxo.value_sats) // using checked_add to prevent overflow
                .ok_or_else(|| CoinSelectionError::new("AMOUNT_OVERFLOW", "Input overflow"))?;

            selected.push(utxo.clone());

            // first we consider the case where we include change.
            // We estimate the fee for the current selection of coins, assuming we will include change in the transaction.
            let (fee_with_change, vbytes) = estimate_fee(
                &selected,
                payments,
                true,
                change.script_type,
                fee_rate_sat_vb,
            );

            // we compute the total required amount, which is the sum of the payment amount and the fee with change.
            // This represents the total amount we need to cover with our selected inputs if we include change.
            let required = payment_sum
                .checked_add(fee_with_change)
                .ok_or_else(|| CoinSelectionError::new("AMOUNT_OVERFLOW", "Required overflow"))?;

            // if the total input value is sufficient to cover the required amount with change, we then check if
            // the change value is above the dust threshold. If it is, we have found a valid solution.
            if total_input >= required {
                let change_value = total_input - required;

                if change_value < DUST_THRESHOLD {
                    continue;
                }

                let total_fee = total_input - payment_sum - change_value;

                // we create a candidate solution with the current selection of coins, total input value, total fee, change
                // included, change value, and vbytes.
                // we then compare this candidate solution with the best solution found so far, and if it is better, we
                // update the best solution.
                let candidate = CoinSelectionResult {
                    selected_coins: selected.clone(),
                    total_input_value: total_input,
                    total_fee,
                    change_included: true,
                    change_value,
                    vbytes,
                };

                match &best {
                    None => best = Some(candidate),
                    Some(current_best) => {
                        if is_better(&candidate, current_best) {
                            best = Some(candidate);
                        }
                    }
                }

                break;
            }
        }
    }

    // Finally we return the best solution found after all iterations, or an error if no valid solution was found.
    best.ok_or_else(|| {
        CoinSelectionError::new("KNAPSACK_FAILED", "Knapsack could not find solution")
    })
}

// This function compares two coin selection results and determines if the first one is better than the second one
// based on the following criteria:
// 1. Lower total fee is better
// 2. If total fees are equal, lower change value is better
// 3. If total fees and change value are equal, fewer selected coins is better
fn is_better(a: &CoinSelectionResult, b: &CoinSelectionResult) -> bool {
    if a.total_fee < b.total_fee {
        return true;
    }

    if a.total_fee == b.total_fee {
        if a.change_value < b.change_value {
            return true;
        }

        if a.change_value == b.change_value {
            return a.selected_coins.len() < b.selected_coins.len();
        }
    }

    false
}
