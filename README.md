# CoinSmith -- Safe PSBT Transaction Builder

CoinSmith is a Bitcoin wallet engineering tool that constructs **safe
Partially Signed Bitcoin Transactions (PSBTs)** from a set of UTXOs and
payment instructions.

The system validates input fixtures, performs **coin selection using
multiple strategies**, builds an **unsigned Bitcoin transaction**, and
exports a **BIP-174 compliant PSBT** along with a detailed JSON report
explaining the transaction.

The project emphasizes **correctness, wallet safety, and deterministic
fee handling**.

------------------------------------------------------------------------

# Features

## Transaction Construction

-   Builds **unsigned Bitcoin transactions**
-   Exports **BIP-174 PSBT (base64)**
-   Ensures balance correctness
    sum(inputs) = sum(outputs) + fee

------------------------------------------------------------------------

## Coin Selection Strategies

Multiple algorithms are implemented to select the optimal set of inputs:

-   Largest First
-   Smallest First
-   Branch and Bound (BnB)
-   Knapsack

Each strategy produces a candidate result.\
A scoring function evaluates them based on:

-   total fee
-   number of inputs
-   change efficiency
-   script diversity

The best candidate is selected automatically.

------------------------------------------------------------------------

## UTXO Consolidation

When network fees are low, the system attempts to consolidate smaller
UTXOs if doing so reduces long-term transaction costs.

The consolidation heuristic considers:

-   incremental fee increase
-   future spending cost
-   dust thresholds
-   input limits

------------------------------------------------------------------------

# Wallet Safety Rules

## Dust Protection

Outputs below the dust threshold are not created.

    dust threshold = 546 sats

## Change Handling

-   At most **one change output**
-   Change created only if not dust
-   If change would be dust, the value is added to the fee

## Fee Calculation

Transaction fees are calculated using estimated **vbytes**:

    fee = ceil(fee_rate * vbytes)

------------------------------------------------------------------------

# RBF and Locktime

The transaction builder correctly sets sequence numbers and locktime.

### Replace-By-Fee (BIP-125)

  Condition          nSequence
  ------------------ ------------
  rbf=true           0xFFFFFFFD
  locktime present   0xFFFFFFFE
  otherwise          0xFFFFFFFF

### Locktime Types

  Range          Type
  -------------- ----------------
  0              none
  \< 500000000   block height
  ≥ 500000000    unix timestamp

------------------------------------------------------------------------

# Output Report

The builder outputs a machine-readable JSON report containing:

-   selected inputs
-   outputs
-   change output index
-   fee
-   fee rate
-   transaction size (vbytes)
-   RBF signaling status
-   locktime information
-   PSBT (base64)

Example:

``` json
{
  "ok": true,
  "network": "mainnet",
  "strategy": "BnB",
  "fee_sats": 700,
  "fee_rate_sat_vb": 5,
  "vbytes": 140,
  "rbf_signaling": true,
  "locktime": 850000,
  "locktime_type": "block_height"
}
```

------------------------------------------------------------------------

# CLI Usage

Build a transaction from a fixture file:

    cargo run fixture.json output.json

The generated file will contain the transaction report and PSBT.

------------------------------------------------------------------------

# Web Interface

A small web dashboard is included for visualizing transactions.\
It allows users to load a fixture, inspect selected inputs and outputs,
and review fee and safety warnings.

------------------------------------------------------------------------

# API

The backend exposes a simple API:

    POST /build

Builds a PSBT from a fixture JSON.

    GET /api/health

Returns:

``` json
{ "ok": true }
```

------------------------------------------------------------------------

# Setup

Install dependencies:

    ./setup.sh

Start the web application:

    ./web.sh

------------------------------------------------------------------------

# Testing

Run unit tests:

    cargo test

Tests cover:

-   coin selection
-   fee and change handling
-   transaction construction
-   PSBT structure

------------------------------------------------------------------------

# Technologies

-   Rust
-   bitcoin-rs
-   Axum
-   React + Vite

------------------------------------------------------------------------

# Design Goals

The system prioritizes:

-   deterministic transaction construction
-   wallet safety guarantees
-   extensible coin selection strategies
-   protocol-compliant PSBT generation
