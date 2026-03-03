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
    let dust_threshold: u64 = 546;

    let mut total_payment: u64 = 0;
    for payment in payments {
        total_payment = total_payment
            .checked_add(payment.value_sats)
            .ok_or_else(|| CoinSelectionError::new("AMOUNT_OVERFLOW", "Payment sum overflowed"))?;
    }

    let mut total_input: u64 = 0;
    let mut selected_coins: Vec<ValidatedUtxo> = Vec::new();

    for input in sorted_inputs {
        if selected_coins.len() as u32 >= max_inputs {
            return Err(CoinSelectionError::new(
                "LIMIT_REACHED",
                "Insufficient input value within limit",
            ));
        }

        total_input = total_input
            .checked_add(utxos[input.0].value_sats)
            .ok_or_else(|| CoinSelectionError::new("AMOUNT_OVERFLOW", "Input sum overflowed"))?;

        selected_coins.push(utxos[input.0].clone());

        let (fee_with_change, vbytes) = estimate_fee(
            &selected_coins,
            payments,
            true,
            change.script_type,
            fee_rate_sat_vb,
        );

        let required_with_change = total_payment.checked_add(fee_with_change).ok_or_else(|| {
            CoinSelectionError::new(
                "AMOUNT_OVERFLOW",
                "Overflow computing required amount with change",
            )
        })?;

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
        } else {
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

    Err(CoinSelectionError::new(
        "INSUFFICIENT_INPUTS",
        "Total sum of inputs is insufficient to make payment",
    ))
}
