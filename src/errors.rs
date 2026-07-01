//! Error taxonomy for the `financial-math` library.
//!
//! Every fallible operation in this crate returns `Result<T, MathError>`
//! instead of panicking, so that host contracts (e.g. `vault-factory`) can
//! decide how to surface a failure to the caller rather than aborting the
//! whole invocation. The single exception is `#[contractimpl]` wrapper
//! functions in [`crate::lib`], which unwrap into `panic_with_error!` per
//! Soroban convention at the contract boundary.

use soroban_sdk::contracterror;

/// Comprehensive failure states for fixed-point math and approximation
/// algorithms. Discriminants are stable and must never be reordered or
/// reused once shipped, since they are part of the on-chain ABI.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum MathError {
    /// A signed 128-bit intermediate or final result overflowed.
    Overflow = 1,
    /// A signed 128-bit intermediate or final result underflowed
    /// (went below `i128::MIN` or below zero where an unsigned
    /// semantic value, e.g. a Wad price, is expected).
    Underflow = 2,
    /// Division where the divisor was zero.
    DivisionByZero = 3,
    /// An operand was negative where the algorithm's domain requires a
    /// non-negative value (e.g. `sqrt`, `ln` of a non-positive number).
    NegativeInput = 4,
    /// Input fell outside the convergence domain of a Taylor/Newton
    /// approximation (e.g. `ln(x)` requires `x > 0`, `exp(x)` requires
    /// a bounded exponent to avoid overflowing the fixed-point range).
    OutOfDomain = 5,
    /// The requested decimal scale (source or destination) exceeds
    /// [`crate::fixed_point::MAX_SCALE`] or underflows below zero.
    InvalidScale = 6,
    /// An iterative approximation (Taylor series, Newton-Raphson) did not
    /// converge to the requested precision within the configured maximum
    /// iteration budget.
    ConvergenceFailure = 7,
    /// The precision/iteration parameter supplied by the caller was zero
    /// or exceeded the algorithm's safe maximum.
    InvalidPrecision = 8,
}
