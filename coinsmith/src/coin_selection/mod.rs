use crate::input_validation::types::{
    ScriptType, ValidatedChange, ValidatedPayment, ValidatedUtxo,
};
use std::u64;
use strategies::{
    branch_and_bound::select_coins_branch_and_bound, greedy::select_coins_greedy,
    knapsack::select_coins_stochastic_knapsack,
};

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

pub struct LargestFirst;
pub struct SmallesFirst;
pub struct BnB;
pub struct Knapsack;

pub enum SortType {
    ASC,
    DESC,
}

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
