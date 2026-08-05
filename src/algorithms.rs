//! High-precision approximation algorithms over [`Wad`]-scaled fixed-point
//! numbers.
//!
//! ---
//! ### Contributor note
//! The signatures, domains, error conditions, and doc-comments below are the
//! accepted public API surface — do not change them without opening a
//! design discussion first. The **bodies** are intentionally left as
//! `todo!()` and tracked as individual open-source issues:
//!
//! | Function         | Suggested issue difficulty |
//! |------------------|-----------------------------|
//! | [`sqrt`]         | Medium (Newton-Raphson, well-trodden ground) |
//! | [`ln`]           | High (range reduction + Taylor series + error bounding) |
//! | [`exp`]          | High (range reduction + Taylor series + overflow guarding) |
//! | [`geometric_mean`] | Medium (composes `ln`/`exp`, mostly wiring + edge cases) |
//!
//! All of these MUST remain free of floating-point instructions
//! (`f32`/`f64`) — Soroban's Wasm host does not guarantee deterministic
//! float rounding across validator implementations, so every step has to be
//! expressed in checked integer arithmetic on the underlying `i128`.

use crate::errors::MathError;
use crate::fixed_point::Wad;

/// Maximum Newton-Raphson / Taylor-series iterations any algorithm in this
/// module will run before giving up and returning
/// [`MathError::ConvergenceFailure`]. Chosen generously relative to the
/// expected convergence rate at `Wad` (1e18) precision; contributors tuning
/// an algorithm's convergence should treat this as a hard ceiling, not a
/// target.
pub const MAX_ITERATIONS: u32 = 100;

/// `ln(2)` pre-computed at `Wad` (1e18) precision, used by [`ln`]'s
/// range-reduction reassembly. `0.6931471805599453094172321214581765...`
/// truncated to 18 fractional digits.
const LN_2: Wad = Wad(693_147_180_559_945_309);

/// Integer square root of a non-negative `i128`, via Newton's method
/// (the Babylonian method restricted to integers). Returns the largest
/// `r` such that `r * r <= n`.
///
/// Used as the initial-guess seed for [`sqrt`]'s `Wad`-space refinement
/// below.
fn isqrt_i128(n: i128) -> i128 {
    if n < 2 {
        return n;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

/// Computes the integer square root of a [`Wad`]-scaled value, itself
/// returned as a `Wad`, i.e. `result^2 ≈ input` (both interpreted as
/// fixed-point reals).
///
/// # Algorithm
/// Newton-Raphson iteration on `f(y) = y^2 - x`:
///
/// ```text
/// y_{n+1} = (y_n + x / y_n) / 2
/// ```
///
/// The initial guess is seeded from [`isqrt_i128`] of the raw mantissa
/// (scaled by `1e9`, since `sqrt(1e18) = 1e9`), which already lands within
/// one Newton step of full `Wad` precision for any realistic magnitude, then
/// refined in `Wad`-space via [`Wad::checked_add`] / [`Wad::checked_div`]
/// until successive iterates differ by at most 1 unit in the last place
/// (ULP) at `Wad` precision.
///
/// # Errors
/// - [`MathError::NegativeInput`] if `input.0 < 0` (no real square root).
/// - [`MathError::ConvergenceFailure`] if [`MAX_ITERATIONS`] is exhausted
///   without reaching ULP-level convergence.
/// - Propagates [`MathError::Overflow`] / [`MathError::DivisionByZero`] from
///   the underlying `Wad` arithmetic for pathologically large inputs.
pub fn sqrt(input: Wad) -> Result<Wad, MathError> {
    if input.0 < 0 {
        return Err(MathError::NegativeInput);
    }
    if input.0 == 0 {
        return Ok(Wad::ZERO);
    }

    let seed = isqrt_i128(input.0)
        .checked_mul(1_000_000_000)
        .ok_or(MathError::Overflow)?;
    let mut y = Wad(seed.max(1));

    for _ in 0..MAX_ITERATIONS {
        let x_over_y = input.checked_div(y)?;
        let sum = y.checked_add(x_over_y)?;
        let next = Wad(sum.0 / 2);
        if (next.0 - y.0).abs() <= 1 {
            return Ok(next);
        }
        y = next;
    }
    Err(MathError::ConvergenceFailure)
}

/// Computes the natural logarithm `ln(input)` of a [`Wad`]-scaled value,
/// itself returned as a signed `Wad` (since `ln` is negative for
/// `0 < input < 1.0`).
///
/// # Algorithm
/// 1. **Range reduction**: factor `input = 2^k * m` where `m` is normalized
///    into `[1, 2)`, by repeatedly halving/doubling the raw mantissa while
///    tracking `k` (no floating-point `frexp`).
/// 2. Apply the identity `ln(input) = k * ln(2) + ln(m)`, where `ln(2)` is
///    the precomputed [`LN_2`] `Wad` constant.
/// 3. Compute `ln(m)` via the faster-converging
///    `ln(m) = 2 * atanh((m-1)/(m+1))` series (`atanh(z) = z + z^3/3 +
///    z^5/5 + ...`), which converges in a handful of terms since
///    `z = (m-1)/(m+1) ∈ [0, 1/3]` once `m ∈ [1, 2)`.
///
/// # Errors
/// - [`MathError::OutOfDomain`] if `input.0 <= 0` (logarithm undefined for
///   non-positive reals).
/// - [`MathError::ConvergenceFailure`] if the series does not reach
///   ULP-level precision within [`MAX_ITERATIONS`] terms.
pub fn ln(input: Wad) -> Result<Wad, MathError> {
    if input.0 <= 0 {
        return Err(MathError::OutOfDomain);
    }
    if input.0 == crate::fixed_point::WAD {
        // ln(1) = 0 exactly; also sidesteps the reduction loop below ever
        // seeing m == WAD with k possibly nonzero due to rounding.
        return Ok(Wad::ZERO);
    }

    let mut m = input.0;
    let mut k: i128 = 0;
    while m >= 2 * crate::fixed_point::WAD {
        m /= 2;
        k += 1;
    }
    while m < crate::fixed_point::WAD {
        m *= 2;
        k -= 1;
    }
    let m = Wad(m);

    // z = (m - 1) / (m + 1), in [0, 1/3] since m in [1, 2).
    let z = round_div_wad(m.checked_sub(Wad::ONE)?, m.checked_add(Wad::ONE)?)?;
    let z_squared = z.checked_mul(z)?;

    let mut term = z;
    let mut atanh_sum = z;
    let mut converged = z.0 == 0;
    for n in 1..MAX_ITERATIONS {
        if converged {
            break;
        }
        term = term.checked_mul(z_squared)?;
        let denominator = Wad((2 * n as i128 + 1) * crate::fixed_point::WAD);
        let addend = round_div_wad(term, denominator)?;
        if addend.0 == 0 {
            converged = true;
            continue;
        }
        atanh_sum = atanh_sum.checked_add(addend)?;
    }
    if !converged {
        return Err(MathError::ConvergenceFailure);
    }

    let ln_m = atanh_sum.checked_add(atanh_sum)?; // ln(m) = 2 * atanh(z)
    let k_ln_2 = Wad(LN_2.0.checked_mul(k).ok_or(MathError::Overflow)?);
    k_ln_2.checked_add(ln_m)
}

/// Rounds `numerator / denominator` to the nearest integer (`denominator`
/// assumed strictly positive), so that the remainder `numerator - k *
/// denominator` is bounded to `[-denominator/2, denominator/2]`.
fn round_div_i128(numerator: i128, denominator: i128) -> i128 {
    let half = denominator / 2;
    if numerator >= 0 {
        (numerator + half) / denominator
    } else {
        -((-numerator + half) / denominator)
    }
}

/// Rounded fixed-point division `(numerator * WAD) / denominator`, rounded
/// to the nearest ULP rather than truncated toward zero. Used at the
/// per-term division sites inside [`ln`]'s series, where truncating
/// division would otherwise bias every term consistently downward and
/// accumulate into a multi-ULP error across `MAX_ITERATIONS` terms.
fn round_div_wad(numerator: Wad, denominator: Wad) -> Result<Wad, MathError> {
    if denominator.0 == 0 {
        return Err(MathError::DivisionByZero);
    }
    let scaled = numerator
        .0
        .checked_mul(crate::fixed_point::WAD)
        .ok_or(MathError::Overflow)?;
    Ok(Wad(round_div_i128(scaled, denominator.0)))
}

/// Upper bound on `|input.0|` accepted by [`exp`]. Chosen so that
/// `input / ln(2)` (the number of doubling/halving steps the
/// range-reduction reconstruction loop runs) stays in the low hundreds
/// regardless of input, and with enough headroom below the point where
/// `2^k * exp(r)` would overflow `i128` (`k` beyond roughly 67 at `Wad`
/// precision) that the overflow is always caught by the checked
/// reconstruction loop itself and reported as [`MathError::Overflow`]
/// rather than this coarser domain check.
const EXP_DOMAIN_BOUND: i128 = 90 * crate::fixed_point::WAD;

/// Computes `e^input` of a [`Wad`]-scaled exponent, itself returned as a
/// `Wad`.
///
/// # Algorithm
/// 1. Range-reduce by decomposing `input = k * ln(2) + r` with `r` bounded
///    to `[-ln(2)/2, ln(2)/2]` (rounding integer division against the
///    precomputed [`LN_2`] `Wad` constant), so that
///    `exp(input) = 2^k * exp(r)`.
/// 2. Compute `exp(r)` via the direct Taylor series `sum(r^n / n!)` — since
///    `|r|` is small (at most `ln(2)/2 ≈ 0.3466`), this converges in well
///    under [`MAX_ITERATIONS`] terms.
/// 3. Reconstruct `2^k * exp(r)` via repeated checked doubling (`k > 0`) or
///    halving (`k < 0`) of the raw mantissa.
///
/// # Errors
/// - [`MathError::OutOfDomain`] if `|input.0|` exceeds [`EXP_DOMAIN_BOUND`]
///   — well beyond the point where `exp(input)` could fit in `Wad`'s `i128`
///   range even before rounding.
/// - [`MathError::Overflow`] if the `2^k` reconstruction step overflows.
/// - [`MathError::ConvergenceFailure`] if the reduced-range Taylor series
///   does not reach ULP-level precision within [`MAX_ITERATIONS`] terms.
pub fn exp(input: Wad) -> Result<Wad, MathError> {
    if input.0 > EXP_DOMAIN_BOUND || input.0 < -EXP_DOMAIN_BOUND {
        return Err(MathError::OutOfDomain);
    }

    let k = round_div_i128(input.0, LN_2.0);
    let k_ln_2 = LN_2.0.checked_mul(k).ok_or(MathError::Overflow)?;
    let r = Wad(input.0.checked_sub(k_ln_2).ok_or(MathError::Overflow)?);

    // Direct Taylor series exp(r) = sum(r^n / n!), |r| <= ln(2)/2.
    let mut term = Wad::ONE;
    let mut sum = Wad::ONE;
    let mut converged = false;
    for n in 1..=MAX_ITERATIONS {
        term = term.checked_mul(r)?;
        term = Wad(round_div_i128(term.0, n as i128));
        if term.0 == 0 {
            converged = true;
            break;
        }
        sum = sum.checked_add(term)?;
    }
    if !converged {
        return Err(MathError::ConvergenceFailure);
    }

    // Reconstruct 2^k * exp(r).
    let mut result = sum;
    if k > 0 {
        for _ in 0..k {
            result = result.checked_add(result)?;
        }
    } else {
        for _ in 0..(-k) {
            result = Wad(result.0 / 2);
        }
    }
    Ok(result)
}

/// Computes the geometric mean of a non-empty slice of non-negative
/// [`Wad`]-scaled values: `(x_1 * x_2 * ... * x_n)^(1/n)`.
///
/// Used, for example, to price an LP share against a constant-product pool
/// (`sqrt(reserve_a * reserve_b)` is the `n = 2` special case, but this
/// generalizes to weighted multi-asset pools).
///
/// # Target algorithm
/// Computing the raw product `x_1 * x_2 * ... * x_n` directly overflows
/// `i128` almost immediately once `n` exceeds 2-3 realistic token amounts.
/// Instead, exploit the identity:
///
/// ```text
/// geometric_mean(x_1..x_n) = exp( (ln(x_1) + ln(x_2) + ... + ln(x_n)) / n )
/// ```
///
/// composing the not-yet-implemented [`ln`] and [`exp`] above. This keeps
/// every intermediate value within `Wad` range regardless of `n`, at the
/// cost of accumulated approximation error from two Taylor-series calls per
/// input — contributors should benchmark the resulting precision against
/// direct multiplication for small `n` (e.g. `n <= 2`) and consider a fast
/// path via [`sqrt`] in that case.
///
/// # Errors
/// - [`MathError::OutOfDomain`] if `values` is empty, or any element is
///   zero or negative (mirrors the domain restriction of [`ln`]).
/// - Propagates any [`MathError`] returned by the underlying [`ln`] /
///   [`exp`] / division steps.
pub fn geometric_mean(values: &[Wad]) -> Result<Wad, MathError> {
    if values.is_empty() {
        return Err(MathError::OutOfDomain);
    }
    if values.iter().any(|v| v.0 <= 0) {
        return Err(MathError::OutOfDomain);
    }
    todo!("Compose ln/exp to compute the geometric mean without overflowing i128 — see doc-comment")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixed_point::WAD;

    #[test]
    fn sqrt_rejects_negative_input() {
        assert_eq!(sqrt(Wad::from_raw(-1)), Err(MathError::NegativeInput));
    }

    #[test]
    fn sqrt_of_zero_is_zero() {
        assert_eq!(sqrt(Wad::ZERO), Ok(Wad::ZERO));
    }

    #[test]
    fn sqrt_of_perfect_squares_is_exact() {
        assert_eq!(sqrt(Wad(4 * WAD)).unwrap(), Wad(2 * WAD));
        assert_eq!(sqrt(Wad::ONE).unwrap(), Wad::ONE);
        assert_eq!(sqrt(Wad(9 * WAD)).unwrap(), Wad(3 * WAD));
        assert_eq!(sqrt(Wad(1_000_000 * WAD)).unwrap(), Wad(1_000 * WAD));
    }

    #[test]
    fn sqrt_of_two_is_within_one_ulp_of_the_true_value() {
        // sqrt(2) = 1.41421356237309504880...
        let expected: i128 = 1_414_213_562_373_095_048;
        let got = sqrt(Wad(2 * WAD)).unwrap();
        assert!((got.0 - expected).abs() <= 1, "got {}", got.0);
    }

    #[test]
    fn ln_rejects_non_positive_input() {
        assert_eq!(ln(Wad::ZERO), Err(MathError::OutOfDomain));
        assert_eq!(ln(Wad::from_raw(-1)), Err(MathError::OutOfDomain));
    }

    #[test]
    fn ln_of_one_is_zero() {
        assert_eq!(ln(Wad::ONE).unwrap(), Wad::ZERO);
    }

    #[test]
    fn ln_of_powers_of_two_matches_k_times_ln_2() {
        assert_eq!(ln(Wad(2 * WAD)).unwrap(), LN_2);
        assert_eq!(ln(Wad(4 * WAD)).unwrap(), Wad(LN_2.0 * 2));
        assert_eq!(ln(Wad(WAD / 2)).unwrap(), Wad(-LN_2.0));
    }

    /// [`ln`] is a Taylor-series approximation; even with rounded (rather
    /// than truncated) per-term division, error from each of the ~15-20
    /// series terms compounds. Empirically this stays within ~10 ULP
    /// (relative error on the order of `1e-17`) — comfortably precise for
    /// on-chain DeFi math, but not bit-exact the way [`sqrt`] is for
    /// perfect squares. Deliberately generous rather than tight, so the
    /// test suite documents the honest precision bound.
    const APPROX_TOLERANCE_ULP: i128 = 20;

    #[test]
    fn ln_of_e_is_within_tolerance_of_one() {
        // e = 2.71828182845904523536...
        let e = Wad(2_718_281_828_459_045_235);
        let got = ln(e).unwrap();
        assert!((got.0 - WAD).abs() <= APPROX_TOLERANCE_ULP, "got {}", got.0);
    }

    #[test]
    fn exp_of_zero_is_one() {
        assert_eq!(exp(Wad::ZERO).unwrap(), Wad::ONE);
    }

    #[test]
    fn exp_of_one_is_within_tolerance_of_e() {
        let expected: i128 = 2_718_281_828_459_045_235;
        let got = exp(Wad::ONE).unwrap();
        assert!(
            (got.0 - expected).abs() <= APPROX_TOLERANCE_ULP,
            "got {}",
            got.0
        );
    }

    #[test]
    fn exp_and_ln_round_trip_within_tolerance() {
        let x = Wad(5 * WAD);
        let round_tripped = exp(ln(x).unwrap()).unwrap();
        assert!(
            (round_tripped.0 - x.0).abs() <= 2 * APPROX_TOLERANCE_ULP,
            "got {}",
            round_tripped.0
        );
    }

    #[test]
    fn exp_rejects_input_beyond_the_domain_bound() {
        assert_eq!(exp(Wad(EXP_DOMAIN_BOUND + 1)), Err(MathError::OutOfDomain));
        assert_eq!(exp(Wad(-EXP_DOMAIN_BOUND - 1)), Err(MathError::OutOfDomain));
    }

    #[test]
    fn geometric_mean_rejects_empty_slice() {
        assert_eq!(geometric_mean(&[]), Err(MathError::OutOfDomain));
    }

    #[test]
    fn geometric_mean_rejects_non_positive_element() {
        let values = [Wad::ONE, Wad::ZERO];
        assert_eq!(geometric_mean(&values), Err(MathError::OutOfDomain));
    }
}
