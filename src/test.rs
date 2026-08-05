//! Contract-level integration test harness for `financial-math`.
//!
//! These tests exercise the crate through the Soroban host (registering the
//! contract in a real [`Env`] and invoking it via the generated client),
//! complementing the pure-Rust `#[test]` units already colocated in
//! [`crate::fixed_point`] and [`crate::algorithms`]. Use this file as the
//! template when adding a test for any new host-facing entrypoint added to
//! [`crate::FinancialMathContract`].

#![cfg(test)]

use crate::{FinancialMathContract, FinancialMathContractClient};
use soroban_sdk::{vec, Env};

/// Registers a fresh instance of the contract in a fresh test `Env` and
/// returns a typed client bound to it. Every test should start from this
/// helper rather than hand-rolling registration, so that harness changes
/// (e.g. enabling diagnostic events) only need to happen in one place.
fn setup() -> FinancialMathContractClient<'static> {
    let env = Env::default();
    let contract_id = env.register(FinancialMathContract, ());
    FinancialMathContractClient::new(&env, &contract_id)
}

#[test]
fn scale_converts_between_decimal_precisions() {
    let client = setup();
    // 1.000000 at 6 decimals (USDC-style) -> 18 decimals.
    let result = client.scale(&1_000_000i128, &6, &18);
    assert_eq!(result, 1_000_000_000_000_000_000i128);
}

#[test]
fn wad_mul_and_wad_div_round_trip() {
    let client = setup();
    const WAD: i128 = 1_000_000_000_000_000_000;
    let product = client.wad_mul(&(3 * WAD), &(2 * WAD));
    assert_eq!(product, 6 * WAD);
    let quotient = client.wad_div(&product, &(2 * WAD));
    assert_eq!(quotient, 3 * WAD);
}

#[test]
#[should_panic]
fn wad_div_by_zero_panics_through_the_host_boundary() {
    let client = setup();
    client.wad_div(&1_000_000_000_000_000_000i128, &0i128);
}

// --- Approximation algorithm entrypoints ------------------------------------
//
// `sqrt`, `ln`, `exp`, and `geometric_mean` are implemented in
// `algorithms.rs` (Newton-Raphson / range-reduced Taylor series / exact
// integer-sqrt fast path respectively); these exercise them through the
// host contract boundary, complementing the pure-Rust unit tests in
// `algorithms.rs` itself.

#[test]
fn sqrt_of_four_is_two() {
    let client = setup();
    const WAD: i128 = 1_000_000_000_000_000_000;
    let result = client.sqrt(&(4 * WAD));
    assert_eq!(result, 2 * WAD);
}

#[test]
#[should_panic]
fn sqrt_of_negative_panics_through_the_host_boundary() {
    let client = setup();
    client.sqrt(&-1i128);
}

#[test]
fn ln_of_one_is_zero() {
    let client = setup();
    const WAD: i128 = 1_000_000_000_000_000_000;
    let result = client.ln(&WAD);
    assert_eq!(result, 0);
}

#[test]
fn exp_of_zero_is_one() {
    let client = setup();
    const WAD: i128 = 1_000_000_000_000_000_000;
    let result = client.exp(&0i128);
    assert_eq!(result, WAD);
}

#[test]
fn geometric_mean_of_matching_values_is_that_value() {
    let client = setup();
    const WAD: i128 = 1_000_000_000_000_000_000;
    // Reuse the client's own `Env` (rather than spinning up a second,
    // unrelated `Env::default()`) — a `Vec` created against a different
    // host instance than the one the client is bound to is a foreign
    // object reference and the host rejects it as "mis-tagged" the moment
    // it's passed as an argument.
    let values = vec![&client.env, 4 * WAD, 4 * WAD];
    let result = client.geometric_mean(&values);
    assert_eq!(result, 4 * WAD);
}

#[test]
fn geometric_mean_of_three_values() {
    let client = setup();
    const WAD: i128 = 1_000_000_000_000_000_000;
    // geometric_mean(1, 2, 4) = 2 (composed via ln/exp for n > 2), within a
    // few ULP of Wad precision.
    let values = vec![&client.env, WAD, 2 * WAD, 4 * WAD];
    let result = client.geometric_mean(&values);
    assert!((result - 2 * WAD).abs() <= 20, "got {}", result);
}
