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

