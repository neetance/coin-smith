pub mod coin_selection;
pub mod input_validation;
pub mod unsigned_tx_builder;

use std::u64;

use coin_selection::{CoinSelectionResult, LargestFirst, SmallesFirst};
use input_validation::validate_raw_fixture;
use unsigned_tx_builder::{PsbtResult, build_unsigned_tx};

use crate::coin_selection::CoinSelectionStrategy;

pub fn run(fixture_raw: &str) -> Result<PsbtResult, (String, String)> {
    let raw_fixture = serde_json::from_str(fixture_raw)
        .map_err(|_| ("INVALID_FIXTURE".to_string(), "Malformed JSON".to_string()))?;

    let validated = validate_raw_fixture(raw_fixture).map_err(|e| (e.code, e.message))?;

    let strategies: Vec<Box<dyn CoinSelectionStrategy>> =
        vec![Box::new(SmallesFirst), Box::new(LargestFirst)];

    let mut coin_selections = Vec::new();
    for strategy in &strategies {
        let coins = strategy
            .select(
                &validated.utxos,
                &validated.payments,
                &validated.change,
                validated.fee_rate_sat_vb,
                validated
                    .policy
                    .as_ref()
                    .and_then(|p| p.max_inputs)
                    .unwrap_or(u32::MAX),
            )
            .map_err(|e| (e.code, e.message))?;
        coin_selections.push(coins);
    }

    let mut strategy = String::new();
    let mut min_score = u64::MAX;
    let mut index = 0;

    for (idx, coins) in coin_selections.iter().enumerate() {
        let score = compute_score(coins);
        if min_score > score {
            min_score = score;
            index = idx;
            strategy = strategies[idx].name().to_string();
        }
    }

    let psbt_result = build_unsigned_tx(
        strategy,
        &validated.payments,
        &coin_selections[index],
        &validated.change,
        validated.rbf,
        validated.locktime,
        validated.current_height,
        validated.fee_rate_sat_vb,
    )
    .map_err(|e| ("TX_BUILD_ERROR".to_string(), e.to_string()))?;

    Ok(psbt_result)
}

fn compute_score(result: &CoinSelectionResult) -> u64 {
    let fee_weight = 1;
    let input_weight = 200;

    (result.total_fee * fee_weight) + (result.selected_coins.len() as u64 * input_weight)
}
