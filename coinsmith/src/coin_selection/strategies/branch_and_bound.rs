use crate::coin_selection::fee_estimator::estimate_fee;
use crate::coin_selection::{CoinSelectionError, CoinSelectionResult};
use crate::input_validation::types::*;

const MAX_BNB_ITERATIONS: usize = 100_000;

pub fn select_coins_branch_and_bound(
    utxos: &[ValidatedUtxo],
    payments: &[ValidatedPayment],
    change: &ValidatedChange,
    fee_rate_sat_vb: f64,
    max_inputs: u32,
) -> Result<CoinSelectionResult, CoinSelectionError> {
    let mut payment_sum: u64 = 0;
    for p in payments {
        payment_sum = payment_sum
            .checked_add(p.value_sats)
            .ok_or_else(|| CoinSelectionError::new("AMOUNT_OVERFLOW", "Payment overflow"))?;
    }

    // Sort descending
    let mut sorted: Vec<ValidatedUtxo> = utxos.to_vec();
    sorted.sort_by(|a, b| b.value_sats.cmp(&a.value_sats));

    let total_available: u64 = sorted.iter().map(|u| u.value_sats).sum();

    let mut iterations = 0usize;
    let mut best_solution: Option<Vec<ValidatedUtxo>> = None;

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

    let (fee_without_change, _) =
        estimate_fee(selected, &[], false, change.script_type, fee_rate_sat_vb);

    let target = match payment_sum.checked_add(fee_without_change) {
        Some(t) => t,
        None => return,
    };

    let (fee_with_change, _) =
        estimate_fee(selected, &[], true, change.script_type, fee_rate_sat_vb);

    let upper_bound = match payment_sum.checked_add(fee_with_change) {
        Some(u) => u,
        None => return,
    };

    if current_value >= target && current_value <= upper_bound {
        *best_solution = Some(selected.clone());
        return;
    }

    if current_value > upper_bound {
        return;
    }

    if current_value + remaining_value < target {
        return;
    }

    if index >= utxos.len() {
        return;
    }

    let utxo = &utxos[index];

    // INCLUDE branch
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

    // EXCLUDE branch
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
