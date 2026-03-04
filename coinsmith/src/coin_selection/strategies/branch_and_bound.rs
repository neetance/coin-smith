/*
 Here we implement a branch and bound coin selection strategy that searches for an exact match to the payment amount + fee.
 This algorithm is based on the one described in the Bitcoin Core codebase, and adapted to our fee estimation and coin
 selection result structures.
*/

use crate::coin_selection::fee_estimator::estimate_fee;
use crate::coin_selection::{CoinSelectionError, CoinSelectionResult};
use crate::input_validation::types::*;

const MAX_BNB_ITERATIONS: usize = 100_000; // We limit the max iterations to prevent long-running searches in large UTXO sets

pub fn select_coins_branch_and_bound(
    utxos: &[ValidatedUtxo],
    payments: &[ValidatedPayment],
    change: &ValidatedChange,
    fee_rate_sat_vb: f64,
    max_inputs: u32,
) -> Result<CoinSelectionResult, CoinSelectionError> {
    // computing the total payment amount
    let mut payment_sum: u64 = 0;
    for p in payments {
        payment_sum = payment_sum
            .checked_add(p.value_sats)
            .ok_or_else(|| CoinSelectionError::new("AMOUNT_OVERFLOW", "Payment overflow"))?;
    }

    // Sort descending
    let mut sorted: Vec<ValidatedUtxo> = utxos.to_vec();
    sorted.sort_by(|a, b| b.value_sats.cmp(&a.value_sats));

    // Precompute the total available value to use as an upper bound in the search
    let total_available: u64 = sorted.iter().map(|u| u.value_sats).sum();

    let mut iterations = 0usize;
    let mut best_solution: Option<Vec<ValidatedUtxo>> = None;

    // Start the branch and bound search
    search(
        0,
        0,
        total_available,
        &mut Vec::new(),
        &sorted,
        payment_sum,
        change,
        fee_rate_sat_vb,
        max_inputs,
        &mut iterations,
        &mut best_solution,
    );

    // If we found a solution, we return it. Otherwise, we return an error indicating that no exact match was found.
    if let Some(solution) = best_solution {
        let total_input: u64 = solution.iter().map(|u| u.value_sats).sum();

        let (_, vbytes) = estimate_fee(
            &solution,
            payments,
            false,
            change.script_type,
            fee_rate_sat_vb,
        );

        return Ok(CoinSelectionResult {
            selected_coins: solution,
            total_input_value: total_input,
            total_fee: total_input - payment_sum,
            change_included: false,
            change_value: 0,
            vbytes,
        });
    }

    Err(CoinSelectionError::new(
        "NO_BNB_SOLUTION",
        "No exact-match solution found",
    ))
}

// This function performs the recursive branch and bound search. For each utxo, we either include it in the selection
// or exclude it, and we compute the current value of the selection and the remaining value of the utxos to decide
// whether to continue searching down that branch or to prune it.
fn search(
    index: usize,
    current_value: u64,
    remaining_value: u64,
    selected: &mut Vec<ValidatedUtxo>,
    utxos: &[ValidatedUtxo],
    payment_sum: u64,
    change: &ValidatedChange,
    fee_rate_sat_vb: f64,
    max_inputs: u32,
    iterations: &mut usize,
    best_solution: &mut Option<Vec<ValidatedUtxo>>,
) {
    if *iterations > MAX_BNB_ITERATIONS {
        return;
    }
    *iterations += 1;

    if selected.len() as u32 > max_inputs {
        return;
    }

    // Compute the fee for the current selection to determine the target amount we need to match (payment + fee)
    let (fee_without_change, _) =
        estimate_fee(selected, &[], false, change.script_type, fee_rate_sat_vb);

    // The target is the payment sum plus the fee without change, which represents the minimum amount we need to match to have a valid solution.
    let target = match payment_sum.checked_add(fee_without_change) {
        Some(t) => t,
        None => return,
    };

    // The upper bound is the payment sum plus the fee with change, which represents the maximum amount we would consider as a valid solution.
    // If the current value exceeds this upper bound, we can prune this branch.
    let (fee_with_change, _) =
        estimate_fee(selected, &[], true, change.script_type, fee_rate_sat_vb);

    let upper_bound = match payment_sum.checked_add(fee_with_change) {
        Some(u) => u,
        None => return,
    };

    // If the current value is between the target and the upper bound, we have found a valid solution. We can store it as the best solution
    // and stop searching further down this branch.
    if current_value >= target && current_value <= upper_bound {
        *best_solution = Some(selected.clone());
        return;
    }

    // If the current value exceeds the upper bound, or if even adding all remaining utxos we cannot reach the target, we can prune this branch.
    if current_value > upper_bound {
        return;
    }

    // Or if even adding all remaining utxos we cannot reach the target, we can prune this branch.
    if current_value + remaining_value < target {
        return;
    }

    // If we have exhausted all utxos, we return.
    if index >= utxos.len() {
        return;
    }

    let utxo = &utxos[index];

    // Here we create a new branch where we include the current utxo in the selection, and we continue searching down that branch.
    if selected.len() as u32 + 1 <= max_inputs {
        selected.push(utxo.clone());

        search(
            index + 1,
            current_value + utxo.value_sats,
            remaining_value - utxo.value_sats,
            selected,
            utxos,
            payment_sum,
            change,
            fee_rate_sat_vb,
            max_inputs,
            iterations,
            best_solution,
        );

        selected.pop();

        if best_solution.is_some() {
            return;
        }
    }

    // Here we create a new branch where we exclude the current utxo from the selection, and we continue searching down that branch.
    search(
        index + 1,
        current_value,
        remaining_value - utxo.value_sats,
        selected,
        utxos,
        payment_sum,
        change,
        fee_rate_sat_vb,
        max_inputs,
        iterations,
        best_solution,
    );
}
