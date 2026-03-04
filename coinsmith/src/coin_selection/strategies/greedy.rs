/*
 Here we implement the greedy coin selection strategy, which iterates through the sorted list of UTXOs(by their effective value)
 and selects them one by one until the total input value is sufficient to cover the payment amount plus fees. This strategy is
 simple and efficient, but it may not always yield the optimal solution in terms of minimizing change or fees.
*/

use crate::coin_selection::fee_estimator::estimate_fee;
use crate::coin_selection::{CoinSelectionError, CoinSelectionResult};
use crate::input_validation::types::*;

pub fn select_coins_greedy(
    utxos: &[ValidatedUtxo],
    sorted_inputs: Vec<(usize, u64)>,
    payments: &[ValidatedPayment],
    change: &ValidatedChange,
    fee_rate_sat_vb: f64,
    max_inputs: u32,
) -> Result<CoinSelectionResult, CoinSelectionError> {
    // we define the dust threshold, which is the minimum change value we would consider including in the transaction.
    // If the change is below this threshold, we will not include it and instead add it to the fee.
    let dust_threshold: u64 = 546;

    // we compute the total payment amount, which is the sum of all payment values. This is the target amount we need to
    // cover with our selected inputs plus fees.
    let mut total_payment: u64 = 0;
    for payment in payments {
        total_payment = total_payment
            .checked_add(payment.value_sats)
            .ok_or_else(|| CoinSelectionError::new("AMOUNT_OVERFLOW", "Payment sum overflowed"))?;
    }

    let mut total_input: u64 = 0;
    let mut selected_coins: Vec<ValidatedUtxo> = Vec::new();

    for input in sorted_inputs {
        // if we have already selected the maximum number of inputs allowed by the policy, we stop and return an error
        // indicating that we cannot find a valid selection within the input limit.
        if selected_coins.len() as u32 >= max_inputs {
            return Err(CoinSelectionError::new(
                "LIMIT_REACHED",
                "Insufficient input value within limit",
            ));
        }

        // we add the value of the current input to the total input sum, and we add the corresponding UTXO to the list of selected coins.
        total_input = total_input
            .checked_add(utxos[input.0].value_sats) // using checked_add to prevent overflow
            .ok_or_else(|| CoinSelectionError::new("AMOUNT_OVERFLOW", "Input sum overflowed"))?;

        selected_coins.push(utxos[input.0].clone());

        // first, we consider the case where we include change. We estimate the fee for the current selection of coins,
        // assuming we will include change in the transaction.
        let (fee_with_change, vbytes) = estimate_fee(
            &selected_coins,
            payments,
            true,
            change.script_type,
            fee_rate_sat_vb,
        );

        // we compute the total required amount, which is the sum of the payment amount and the fee with change.
        // This represents the total amount we need to cover with our selected inputs if we include change.
        let required_with_change = total_payment.checked_add(fee_with_change).ok_or_else(|| {
            CoinSelectionError::new(
                "AMOUNT_OVERFLOW",
                "Overflow computing required amount with change",
            )
        })?;

        // if the total input value is sufficient to cover the required amount with change, we then check if the change
        // value would be above the dust threshold.
        // if the change value is above the dust threshold, we can return this selection as a valid solution, including the
        // change output.
        if total_input >= required_with_change {
            let change_value = total_input
                .checked_sub(required_with_change)
                .ok_or_else(|| {
                    CoinSelectionError::new("AMOUNT_UNDERFLOW", "Underflow computing change")
                })?;

            if change_value >= dust_threshold {
                return Ok(CoinSelectionResult {
                    selected_coins,
                    total_input_value: total_input,
                    total_fee: fee_with_change,
                    change_included: true,
                    change_value,
                    vbytes,
                });
            }

            // if the change value is below the dust threshold, we will not include a change output. Instead, we will
            // add the would-be change value to the fee,
            let (fee_without_change, vbytes) = estimate_fee(
                &selected_coins,
                payments,
                false,
                change.script_type,
                fee_rate_sat_vb,
            );

            let required_without_change = total_payment
                .checked_add(fee_without_change)
                .ok_or_else(|| {
                    CoinSelectionError::new(
                        "AMOUNT_OVERFLOW",
                        "Overflow computing required amount without change",
                    )
                })?;

            // we check if the total input value is sufficient to cover the required amount without change.
            // If it is, we return this selection as a valid solution, without including change.
            if total_input >= required_without_change {
                return Ok(CoinSelectionResult {
                    selected_coins,
                    total_input_value: total_input,
                    total_fee: total_input - total_payment,
                    change_included: false,
                    change_value: 0,
                    vbytes,
                });
            }
        }
        // if the total input value is not sufficient to cover the required amount with change, we then consider
        // the case where we do not include change.
        else {
            let (fee_without_change, vbytes) = estimate_fee(
                &selected_coins,
                payments,
                false,
                change.script_type,
                fee_rate_sat_vb,
            );

            let required_without_change = total_payment
                .checked_add(fee_without_change)
                .ok_or_else(|| {
                    CoinSelectionError::new(
                        "AMOUNT_OVERFLOW",
                        "Overflow computing required amount without change",
                    )
                })?;

            // we check if the total input value is sufficient to cover the required amount without change. If it is,
            // we return this selection as a valid solution, without including change.
            if total_input >= required_without_change {
                return Ok(CoinSelectionResult {
                    selected_coins,
                    total_input_value: total_input,
                    total_fee: total_input - total_payment,
                    change_included: false,
                    change_value: 0,
                    vbytes,
                });
            }
        }
    }

    // at last, if we have iterated through all inputs and we have not found a valid selection that can cover
    // the payment amount plus fees, we return an error indicating that the total sum of inputs is insufficient
    // to make the payment.
    Err(CoinSelectionError::new(
        "INSUFFICIENT_INPUTS",
        "Total sum of inputs is insufficient to make payment",
    ))
}
