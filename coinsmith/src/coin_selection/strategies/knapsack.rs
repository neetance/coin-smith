use crate::coin_selection::fee_estimator::estimate_fee;
use crate::coin_selection::{CoinSelectionError, CoinSelectionResult};
use crate::input_validation::types::*;
use rand::seq::SliceRandom;
use rand::thread_rng;

pub fn select_coins_stochastic_knapsack(
    utxos: &[ValidatedUtxo],
    payments: &[ValidatedPayment],
    change: &ValidatedChange,
    fee_rate_sat_vb: f64,
    max_inputs: u32,
) -> Result<CoinSelectionResult, CoinSelectionError> {
    const KNAPSACK_ITERATIONS: usize = 2000;
    const DUST_THRESHOLD: u64 = 546;

    let mut payment_sum: u64 = 0;
    for p in payments {
        payment_sum = payment_sum
            .checked_add(p.value_sats)
            .ok_or_else(|| CoinSelectionError::new("AMOUNT_OVERFLOW", "Payment overflow"))?;
    }

    let mut best: Option<CoinSelectionResult> = None;
    let mut rng = thread_rng();

    for _ in 0..KNAPSACK_ITERATIONS {
        let mut shuffled = utxos.to_vec();
        shuffled.shuffle(&mut rng);

        let mut selected = Vec::new();
        let mut total_input: u64 = 0;

        for utxo in shuffled.iter() {
            if selected.len() as u32 >= max_inputs {
                break;
            }

            total_input = total_input
                .checked_add(utxo.value_sats)
                .ok_or_else(|| CoinSelectionError::new("AMOUNT_OVERFLOW", "Input overflow"))?;

            selected.push(utxo.clone());

            let (fee_with_change, vbytes) = estimate_fee(
                &selected,
                payments,
                true,
                change.script_type,
                fee_rate_sat_vb,
            );

            let required = payment_sum
                .checked_add(fee_with_change)
                .ok_or_else(|| CoinSelectionError::new("AMOUNT_OVERFLOW", "Required overflow"))?;

            if total_input >= required {
                let change_value = total_input - required;

                if change_value < DUST_THRESHOLD {
                    continue;
                }

                let total_fee = total_input - payment_sum - change_value;

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

    best.ok_or_else(|| {
        CoinSelectionError::new("KNAPSACK_FAILED", "Knapsack could not find solution")
    })
}

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
