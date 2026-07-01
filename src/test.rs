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

