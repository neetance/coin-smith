/*
 This module contains the main coin selection logic, and defines the structs and traits which we will be using
 throughout the coin selection process. It also contains the main function for each strategy, which will be called
 from the main function in lib.rs. The actual implementation of each strategy is in the strategies submodule,
 and we also have a fee estimator module for estimating the fees for a given selection of coins.
*/

use crate::input_validation::types::{
    ScriptType, ValidatedChange, ValidatedPayment, ValidatedUtxo,
};
use std::u64;
use strategies::{
    branch_and_bound::select_coins_branch_and_bound, greedy::select_coins_greedy,
    knapsack::select_coins_stochastic_knapsack,
};

// This struct represents the result of a coin selection process, which includes the selected coins, the total input value,
// the total fee, whether change is included or not, the change value if change is included, and the estimated size of the
// transaction in vbytes.
pub struct CoinSelectionResult {
    pub selected_coins: Vec<ValidatedUtxo>,
    pub total_input_value: u64,
    pub total_fee: u64,
    pub change_included: bool,
    pub change_value: u64,
    pub vbytes: usize,
}

impl Clone for CoinSelectionResult {
    fn clone(&self) -> Self {
        return Self {
            selected_coins: self.selected_coins.clone(),
            total_input_value: self.total_input_value,
            total_fee: self.total_fee,
            change_included: self.change_included,
            change_value: self.change_value,
            vbytes: self.vbytes,
        };
    }
}

// This struct represents an error that can occur during the coin selection process, which includes an error code and a message.
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

// This trait defines the interface for a coin selection strategy, which includes a select function that returns the result of
// the coin selection process, and a name function that returns the name of the strategy.
// The different strategies will implement this trait, and we will call the select function from the main function in lib.rs
// to perform the coin selection.
pub trait CoinSelectionStrategy {
    fn select(
        &self,
        utxos: &[ValidatedUtxo],
        payments: &[ValidatedPayment],
        change: &ValidatedChange,
        fee_rate_sat_vb: f64,
        max_inputs: u32,
    ) -> Result<CoinSelectionResult, CoinSelectionError>;
    fn name(&self) -> &'static str;
}

// Structs for the different strategies that will implement the CoinSelectionStrategy trait.
pub struct LargestFirst;
pub struct SmallesFirst;
pub struct BnB;
pub struct Knapsack;

pub enum SortType {
    ASC,
    DESC,
}

// Here we implement CoinSelectionStrategy for the LargestFirst and SmallestFirst strategies, which both use the same greedy
// selection logic, but differ in the order in which they sort the UTXOs (largest first or smallest first). The select function
// for both strategies calls the select_coins_greedy function, which contains the main logic for the greedy coin selection
// algorithm. The name function returns the name of the strategy, which will be used in the main function in lib.rs to identify
// which strategy produced which result.
impl CoinSelectionStrategy for LargestFirst {
    fn select(
        &self,
        utxos: &[ValidatedUtxo],
        payments: &[ValidatedPayment],
        change: &ValidatedChange,
        fee_rate_sat_vb: f64,
        max_inputs: u32,
    ) -> Result<CoinSelectionResult, CoinSelectionError> {
        let sorted_inputs = sort_utxos_by_input_value(utxos, SortType::DESC, fee_rate_sat_vb);
        return select_coins_greedy(
            utxos,
            sorted_inputs,
            payments,
            change,
            fee_rate_sat_vb,
            max_inputs,
        );
    }

    fn name(&self) -> &'static str {
        return "greedy(largest_first)";
    }
}

impl CoinSelectionStrategy for SmallesFirst {
    fn select(
        &self,
        utxos: &[ValidatedUtxo],
        payments: &[ValidatedPayment],
        change: &ValidatedChange,
        fee_rate_sat_vb: f64,
        max_inputs: u32,
    ) -> Result<CoinSelectionResult, CoinSelectionError> {
        let sorted_inputs = sort_utxos_by_input_value(utxos, SortType::ASC, fee_rate_sat_vb);
        return select_coins_greedy(
            utxos,
            sorted_inputs,
            payments,
            change,
            fee_rate_sat_vb,
            max_inputs,
        );
    }

    fn name(&self) -> &'static str {
        return "greedy(smallest_first)";
    }
}

// Here we implement the CoinSelectionStrategy for branch and bound, which calls the select_coins_branch_and_bound function
// that contains the main logic for the branch and bound coin selection algorithm. The name function returns the name of the
// strategy, which will be used in the main function in lib.rs to identify which strategy produced which result.
impl CoinSelectionStrategy for BnB {
    fn select(
        &self,
        utxos: &[ValidatedUtxo],
        payments: &[ValidatedPayment],
        change: &ValidatedChange,
        fee_rate_sat_vb: f64,
        max_inputs: u32,
    ) -> Result<CoinSelectionResult, CoinSelectionError> {
        return select_coins_branch_and_bound(utxos, payments, change, fee_rate_sat_vb, max_inputs);
    }

    fn name(&self) -> &'static str {
        return "branch_and_bound";
    }
}

// Here we implement the CoinSelectionStrategy for the knapsack strategy, which calls the select_coins_stochastic_knapsack
// function that contains the main logic for the stochastic knapsack coin selection algorithm. The name function returns the
// name of the strategy, which will be used in the main function in lib.rs to identify which strategy produced which result.
impl CoinSelectionStrategy for Knapsack {
    fn select(
        &self,
        utxos: &[ValidatedUtxo],
        payments: &[ValidatedPayment],
        change: &ValidatedChange,
        fee_rate_sat_vb: f64,
        max_inputs: u32,
    ) -> Result<CoinSelectionResult, CoinSelectionError> {
        return select_coins_stochastic_knapsack(
            utxos,
            payments,
            change,
            fee_rate_sat_vb,
            max_inputs,
        );
    }

    fn name(&self) -> &'static str {
        return "stochastic_knapsack";
    }
}

// Helper function to sort the utxos by their effective value, which is the value of the utxo minus the estimated fee to spend it.
// We use this function to sort the utxos in either ascending or descending order, depending on the strategy we are implementing
// (largest first or smallest first).
pub fn sort_utxos_by_input_value(
    utxos: &[ValidatedUtxo],
    sort_type: SortType,
    fee_rate_sat_vb: f64,
) -> Vec<(usize, u64)> {
    let mut sorted_values = Vec::new();
    for (idx, utxo) in utxos.iter().enumerate() {
        let value_sats = utxo.value_sats;
        let input_vbytes = match utxo.script_type {
            ScriptType::P2PKH => 148,
            ScriptType::P2SH_P2WPKH => 90,
            ScriptType::P2WPKH => 68,
            ScriptType::P2TR => 58,
        };
        let spending_cost = input_vbytes * (fee_rate_sat_vb as u64);
        let effective_value = value_sats - spending_cost;

        sorted_values.push((idx, effective_value));
    }

    match sort_type {
        SortType::ASC => sort_asc(&mut sorted_values),
        SortType::DESC => sort_desc(&mut sorted_values),
    };
    sorted_values
}

fn sort_desc(values: &mut [(usize, u64)]) -> &[(usize, u64)] {
    values.sort_by(|a, b| b.1.cmp(&a.1));
    values
}

fn sort_asc(values: &mut [(usize, u64)]) -> &[(usize, u64)] {
    values.sort_by(|a, b| a.1.cmp(&b.1));
    values
}

pub mod fee_estimator;
pub mod strategies;
pub mod utxo_consolidation;
