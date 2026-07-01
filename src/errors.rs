//! Error taxonomy for the `financial-math` library.
//!
//! Every fallible operation in this crate returns `Result<T, MathError>`
//! instead of panicking, so that host contracts (e.g. `vault-factory`) can
//! decide how to surface a failure to the caller rather than aborting the
//! whole invocation. The single exception is `#[contractimpl]` wrapper
//! functions in [`crate::lib`], which unwrap into `panic_with_error!` per
//! Soroban convention at the contract boundary.

use soroban_sdk::contracterror;

