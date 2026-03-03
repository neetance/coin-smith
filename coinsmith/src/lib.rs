pub mod coin_selection;
pub mod input_validation;
pub mod unsigned_tx_builder;
use crate::coin_selection::{
    CoinSelectionStrategy, Knapsack, utxo_consolidation::consolidate_utxos,
};
use coin_selection::{BnB, CoinSelectionResult, LargestFirst, SmallesFirst};
use input_validation::validate_raw_fixture;
use std::collections::{HashMap, HashSet};
use std::u64;
use unsigned_tx_builder::{PsbtResult, build_unsigned_tx};

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

    let bnb_result = BnB.select(
        &validated.utxos,
        &validated.payments,
        &validated.change,
        validated.fee_rate_sat_vb,
        validated
            .policy
            .as_ref()
            .and_then(|p| p.max_inputs)
            .unwrap_or(u32::MAX),
    );
    if let Ok(res) = &bnb_result {
        strategy = BnB.name().to_string();
        min_score = compute_score(
            &res,
            (validated.fee_rate_sat_vb * (res.vbytes as f64)) as u64,
        );
    }

    let knapsack_result = Knapsack.select(
        &validated.utxos,
        &validated.payments,
        &validated.change,
        validated.fee_rate_sat_vb,
        validated
            .policy
            .as_ref()
            .and_then(|p| p.max_inputs)
            .unwrap_or(u32::MAX),
    );
    if let Ok(res) = &knapsack_result {
        let score = compute_score(
            &res,
            (validated.fee_rate_sat_vb * (res.vbytes as f64)) as u64,
        );
        if score < min_score {
            min_score = score;
            strategy = Knapsack.name().to_string();
        }
    }

    let mut index = 0;
    for (idx, coins) in coin_selections.iter().enumerate() {
        let score = compute_score(
            coins,
            (validated.fee_rate_sat_vb * (coins.vbytes as f64)) as u64,
        );
        if min_score > score {
            min_score = score;
            index = idx;
            strategy = strategies[idx].name().to_string();
        }
    }

    let coins = if strategy == Knapsack.name() {
        knapsack_result.unwrap()
    } else if strategy == BnB.name() {
        bnb_result.unwrap()
    } else {
        coin_selections[index].clone()
    };

    let mut selected = HashMap::new();

    for coin in &coins.selected_coins {
        selected.insert(coin.txid, true);
    }
    let coins_after_utxo_consolidation = consolidate_utxos(
        &validated.utxos,
        &validated.payments,
        coins,
        validated.change.script_type,
        validated.fee_rate_sat_vb,
        validated
            .policy
            .as_ref()
            .and_then(|p| p.max_inputs)
            .unwrap_or(u32::MAX),
    )
    .map_err(|e| (e.code, e.message))?;

    let psbt_result = build_unsigned_tx(
        strategy,
        &validated.payments,
        &coins_after_utxo_consolidation,
        &validated.change,
        validated.rbf,
        validated.locktime,
        validated.current_height,
        validated.fee_rate_sat_vb,
    )
    .map_err(|e| ("TX_BUILD_ERROR".to_string(), e.to_string()))?;

    Ok(psbt_result)
}

pub fn compute_score(result: &CoinSelectionResult, required_fee: u64) -> u64 {
    const FUTURE_FEE_RATE: u64 = 20;
    const P2WPKH_INPUT_VBYTES: u64 = 68;
    const DUST_THRESHOLD: u64 = 546;

    let future_input_cost = P2WPKH_INPUT_VBYTES * FUTURE_FEE_RATE;
    let mut score: u64 = 0;
    score = score.saturating_add(result.total_fee);

    let input_penalty = (result.selected_coins.len() as u64).saturating_mul(future_input_cost);
    score = score.saturating_add(input_penalty);

    if result.total_fee > required_fee {
        let excess = result.total_fee - required_fee;
        score = score.saturating_add(excess);
    }

    if result.change_included {
        let change = result.change_value;
        if change < future_input_cost * 2 {
            score = score.saturating_add(future_input_cost * 2);
        }

        if change < DUST_THRESHOLD * 2 {
            score = score.saturating_add(DUST_THRESHOLD);
        }
    } else {
        score = score.saturating_sub(500);
    }

    let mut script_types = HashSet::new();
    for coin in &result.selected_coins {
        script_types.insert(coin.script_type as u8);
    }

    if script_types.len() > 1 {
        score = score.saturating_add(300);
    }

    score
}
