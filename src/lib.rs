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

