pub mod coin_selection;
pub mod input_validation;
pub mod unsigned_tx_builder;

use coin_selection::select_coins;
use input_validation::validate_raw_fixture;
use unsigned_tx_builder::{PsbtResult, build_unsigned_tx};

pub fn run(fixture_raw: &str) -> Result<PsbtResult, (String, String)> {
    let raw_fixture = serde_json::from_str(fixture_raw)
        .map_err(|_| ("INVALID_FIXTURE".to_string(), "Malformed JSON".to_string()))?;

    let validated = validate_raw_fixture(raw_fixture).map_err(|e| (e.code, e.message))?;

    let coins = select_coins(
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

    let psbt_result = build_unsigned_tx(
        &validated.payments,
        &coins,
        &validated.change,
        validated.rbf,
        validated.locktime,
        validated.current_height,
        validated.fee_rate_sat_vb,
    )
    .map_err(|e| ("TX_BUILD_ERROR".to_string(), e.to_string()))?;

    Ok(psbt_result)
}
