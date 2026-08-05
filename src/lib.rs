//! `soroban-financial-math` — fixed-point arithmetic and high-precision
//! approximation algorithms for Soroban DeFi contracts.
//!
//! The crate is deliberately split into two layers:
//!
//! - [`fixed_point`] and [`algorithms`] are plain Rust, free of any
//!   `soroban_sdk::Env` dependency, so they can be unit tested with
//!   ordinary `#[test]` and reused by any downstream contract crate
//!   (e.g. a vault or lending contract) as a plain library dependency.
//! - The `#[contract]` in this file is a thin host-facing wrapper that
//!   lets `financial-math` also be deployed and invoked directly as its
//!   own standalone Soroban contract (useful for integration testing, or
//!   for third parties who only need the math and don't want to vendor
//!   the crate as a Rust dependency).
#![no_std]
extern crate alloc;

pub mod algorithms;
pub mod errors;
pub mod fixed_point;

#[cfg(test)]
mod test;

use alloc::vec::Vec as AllocVec;
use fixed_point::Wad;
use soroban_sdk::{contract, contractimpl, panic_with_error, Env, Vec};

#[contract]
pub struct FinancialMathContract;

#[contractimpl]
impl FinancialMathContract {
    /// Converts `amount` from `from_decimals` to `to_decimals` precision.
    /// Thin host wrapper around [`fixed_point::scale_to`].
    pub fn scale(env: Env, amount: i128, from_decimals: u32, to_decimals: u32) -> i128 {
        fixed_point::scale_to(amount, from_decimals, to_decimals)
            .unwrap_or_else(|err| panic_with_error!(&env, err))
    }

    /// Checked `Wad` multiplication, exposed as a raw `i128` in/out for
    /// callers that do not link against this crate's Rust types directly.
    pub fn wad_mul(env: Env, a: i128, b: i128) -> i128 {
        Wad(a)
            .checked_mul(Wad(b))
            .unwrap_or_else(|err| panic_with_error!(&env, err))
            .0
    }

    /// Checked `Wad` division, exposed as a raw `i128` in/out.
    pub fn wad_div(env: Env, a: i128, b: i128) -> i128 {
        Wad(a)
            .checked_div(Wad(b))
            .unwrap_or_else(|err| panic_with_error!(&env, err))
            .0
    }

    /// Fixed-point square root over a `Wad`-scaled raw `i128`.
    ///
    /// Delegates to [`algorithms::sqrt`] — see that function's doc-comment
    /// for the Newton-Raphson algorithm and error conditions.
    pub fn sqrt(env: Env, input: i128) -> i128 {
        algorithms::sqrt(Wad(input))
            .unwrap_or_else(|err| panic_with_error!(&env, err))
            .0
    }

    /// Natural logarithm over a `Wad`-scaled raw `i128`.
    ///
    /// Delegates to [`algorithms::ln`] — see that function's doc-comment
    /// for the range-reduction/Taylor-series algorithm and error
    /// conditions.
    pub fn ln(env: Env, input: i128) -> i128 {
        algorithms::ln(Wad(input))
            .unwrap_or_else(|err| panic_with_error!(&env, err))
            .0
    }

    /// Exponential function over a `Wad`-scaled raw `i128`.
    ///
    /// Delegates to [`algorithms::exp`] — see that function's doc-comment
    /// for the range-reduction/Taylor-series algorithm and error
    /// conditions.
    pub fn exp(env: Env, input: i128) -> i128 {
        algorithms::exp(Wad(input))
            .unwrap_or_else(|err| panic_with_error!(&env, err))
            .0
    }

    /// Geometric mean over a list of `Wad`-scaled raw `i128` values.
    ///
    /// Delegates to [`algorithms::geometric_mean`] — see that function's
    /// doc-comment for the algorithm (an exact-integer-sqrt fast path for
    /// `n <= 2`, `ln`/`exp` composition otherwise) and error conditions.
    pub fn geometric_mean(env: Env, values: Vec<i128>) -> i128 {
        let wads: AllocVec<Wad> = values.iter().map(Wad).collect();
        algorithms::geometric_mean(&wads)
            .unwrap_or_else(|err| panic_with_error!(&env, err))
            .0
    }
}
