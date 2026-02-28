use crate::input_validation::types::{ValidatedChange, ValidatedPayment, ValidatedUtxo};
use fee_estimator::estimate_fee;

pub struct CoinSelectionResult {
    pub selected_coins: Vec<ValidatedUtxo>,
    pub total_input_value: u64,
    pub total_fee: u64,
    pub change_included: bool,
    pub change_value: u64,
    pub vbytes: usize,
}

#[derive(Debug)]
pub struct CoinSelectionError {
    pub code: String,
    pub message: String,
}

impl CoinSelectionError {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
        }
    }
}

pub fn select_coins(
    utxos: &[ValidatedUtxo],
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

    let sorted_inputs = sort_utxos_by_input_value(utxos);

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
            .checked_add(input.1)
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
                    total_fee: fee_without_change,
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

pub fn sort_utxos_by_input_value(utxos: &[ValidatedUtxo]) -> Vec<(usize, u64)> {
    let mut sorted_values = Vec::new();
    for (idx, utxo) in utxos.iter().enumerate() {
        sorted_values.push((idx, utxo.value_sats));
    }
    sort_desc(&mut sorted_values);
    sorted_values
}

fn sort_desc(values: &mut [(usize, u64)]) -> &[(usize, u64)] {
    values.sort_by(|a, b| b.1.cmp(&a.1));
    values
}

pub mod fee_estimator;
