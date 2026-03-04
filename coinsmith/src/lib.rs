/*
 This is the main interface where we take the raw fixture as input, validate it, run the different coin selection strategies,
 compare their scores and return the final psbt result.
*/

pub mod coin_selection;
pub mod input_validation;
pub mod unsigned_tx_builder;

#[cfg(test)]
mod tests;

use crate::coin_selection::{
    CoinSelectionStrategy, Knapsack, utxo_consolidation::consolidate_utxos,
};
use coin_selection::{BnB, CoinSelectionResult, LargestFirst, SmallesFirst};
use input_validation::validate_raw_fixture;
use std::collections::HashSet;
use std::u64;
use unsigned_tx_builder::{PsbtResult, build_unsigned_tx};

// This is the main function which will be called from the main.rs file, where we will pass the raw fixture as a string
// and will return the final psbt result or an error in case of any error during the process.
pub fn run(fixture_raw: &str) -> Result<PsbtResult, (String, String)> {
    // first we validate the raw fixture and convert it to a validated fixture struct, and if there is any error during
    // the validation process, we return an error with the appropriate message.
    let raw_fixture = serde_json::from_str(fixture_raw)
        .map_err(|_| ("INVALID_FIXTURE".to_string(), "Malformed JSON".to_string()))?;
    let validated = validate_raw_fixture(raw_fixture).map_err(|e| (e.code, e.message))?;

    // then we run the different coin selection strategies on the validated fixture and get the result of each strategy,
    // and if there is any error during the coin selection process, we return an error with the appropriate message.
    let strategies: Vec<Box<dyn CoinSelectionStrategy>> =
        vec![Box::new(LargestFirst), Box::new(SmallesFirst)];

    // we get the coin selections from the largest first and smallest first strategies
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

    // we calculate the coin selection result for the BnB strategy, and if there is no error, we calculate the score
    // for the result and set it as the minimum score for comparison with the other strategies, and we also set the
    // strategy name as BnB for now.
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

    // we calculate the coin selection result for the Knapsack strategy, and if there is no error, we calculate the score
    // for the result and compare it with the minimum score, and if it is less than the minimum score, we set the strategy
    // name as Knapsack and update the minimum score.
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

    // here we compare the current minimum score with the scores of the largest first and smallest first strategies,
    // and we set the strategy name accordingly, and we also set the index of the selected strategy for later use in
    // getting the coin selection result.
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

    // we set the coin selection result based on the selected strategy
    let coins = if strategy == Knapsack.name() {
        knapsack_result.map_err(|e| (e.code, e.message))?
    } else if strategy == BnB.name() {
        bnb_result.map_err(|e| (e.code, e.message))?
    } else {
        coin_selections[index].clone()
    };

    // after we get the coin selection result from the selected strategy, we run the utxo consolidation function on the result
    // to see if we can further optimize the coin selection by consolidating some utxos, and if there is any error during the
    // consolidation process, we return an error with the appropriate message.
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

    // finally, we build the unsigned transaction using the final coin selection result after consolidation, and if there is
    // any error during the transaction building process, we return an error with the appropriate message, otherwise we return
    // the final psbt result.
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

// This function computes the score for a given coin selection result based on various factors
pub fn compute_score(result: &CoinSelectionResult, required_fee: u64) -> u64 {
    const FUTURE_FEE_RATE: u64 = 20; // estimated future fee rate in sats/vb
    const P2WPKH_INPUT_VBYTES: u64 = 68; // estimated vbytes for a P2WPKH input
    const DUST_THRESHOLD: u64 = 546; // dust threshold in satoshis

    // we calculate the future input cost based on the estimated future fee rate and the vbytes
    // for a typical input, and we initialize the score with the total fee of the coin selection result
    let future_input_cost = P2WPKH_INPUT_VBYTES * FUTURE_FEE_RATE;
    let mut score: u64 = 0;
    score = score.saturating_add(result.total_fee);

    // we add a penalty for each input based on the future input cost, since each additional input in the transaction increases
    // the long term cost of the transaction due to higher fees in the future
    let input_penalty = (result.selected_coins.len() as u64).saturating_mul(future_input_cost);
    score = score.saturating_add(input_penalty); // saturating add to prevent overflow

    // if the total fee of the coin selection result is greater than the required fee, we add a penalty for the excess fee, since
    // a higher fee than required is not optimal and indicates that we are overpaying for the transaction
    if result.total_fee > required_fee {
        let excess = result.total_fee - required_fee;
        score = score.saturating_add(excess);
    }

    // if change is included in the coin selection result, we add a penalty based on the change value, since including
    // change can increase the transaction size and fees
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

    // we add a penalty if the coin selection result includes multiple script types, since it can increase the complexity
    // and cost of the transaction
    let mut script_types = HashSet::new();
    for coin in &result.selected_coins {
        script_types.insert(coin.script_type as u8);
    }

    if script_types.len() > 1 {
        score = score.saturating_add(300);
    }

    // we return the final computed score for the coin selection result, where a lower score indicates a better result
    // in terms of cost and efficiency
    score
}
