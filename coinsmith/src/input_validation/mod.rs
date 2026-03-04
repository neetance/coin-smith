/*
 This module deals with validating the raw fixtures that we get from the fixtures folder
 For each input, it validates the fixture defensively and after the validation, returns a
 clean and validated version of the fixture input
*/

use types::*;
use utils::*;

// This function takes in a raw fixture and returns a validated fixture struct in case of no error
pub fn validate_raw_fixture(raw_fixture: RawFixture) -> Result<ValidatedFixture, ValidationError> {
    let network = raw_fixture.network;
    if network != "mainnet".to_string() {
        return Err(ValidationError::new(
            "INVALID_NETWORK",
            "Network must be mainnet",
        ));
    }

    let utxos = raw_fixture.utxos;
    let payments = raw_fixture.payments;
    let change = raw_fixture.change;

    // validating the different components of the fixture and getting their validated versions
    let validated_utxos = validate_utxos(&utxos)?;
    let validated_payments = validate_payments(&payments)?;
    let validated_change = validate_change(&change)?;

    // checking for valid fee rate
    let validated_fee_rate = raw_fixture.fee_rate_sat_vb;
    if validated_fee_rate == 0.0 {
        return Err(ValidationError::new(
            "INVALID_FEE_RATE",
            "Fee rate must be positive",
        ));
    }

    // validating locktime, current_height and policy
    if let Some(locktime) = raw_fixture.locktime {
        if locktime == u32::MAX {
            return Err(ValidationError::new(
                "INVALID_LOCKTIME",
                "locktime value is not reasonable",
            ));
        }
    }

    if let Some(current_height) = raw_fixture.current_height {
        if current_height == 0 {
            return Err(ValidationError::new(
                "INVALID_CURRENT_HEIGHT",
                "current_height must be greater than 0",
            ));
        }

        if current_height > 1_000_000_000 {
            return Err(ValidationError::new(
                "INVALID_CURRENT_HEIGHT",
                "current_height is unrealistically large",
            ));
        }
    }

    let mut policy = None;
    let mut validated_policy = ValidatedPolicy::new();
    if let Some(raw_policy) = raw_fixture.policy {
        if let Some(max_inputs) = raw_policy.max_inputs {
            if max_inputs == 0 {
                return Err(ValidationError::new(
                    "INVALID_MAX_INPUTS",
                    "Max inputs should be positive",
                ));
            }
            validated_policy.add_max_inputs(max_inputs);
            policy = Some(validated_policy);
        }
    }

    // returning the final validated fixture after performing all checks
    Ok(ValidatedFixture {
        network: network,
        utxos: validated_utxos,
        payments: validated_payments,
        change: validated_change,
        fee_rate_sat_vb: validated_fee_rate,
        rbf: raw_fixture.rbf,
        locktime: raw_fixture.locktime,
        current_height: raw_fixture.current_height,
        policy: policy,
    })
}

pub mod types;
pub mod utils;
