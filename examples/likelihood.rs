//! The likelihood of 100,000 observations: a product far below the range of `f64`.
//!
//! Draw 100,000 samples from a normal distribution, then compute the likelihood of the data
//! under two candidate models, Π p(xᵢ | model). Each factor is a density around 0.3, so the
//! product is about 10⁻⁶¹⁴⁴¹. Plain `f64` multiplication reaches 0 after a few hundred factors
//! and stays there, which makes the two models impossible to compare. With `Log` the
//! products are ordinary values: they print, compare, and divide to give a likelihood ratio
//! that is itself back inside the float range.
//!
//! Run with `cargo run --example likelihood`.

use mirifici::Log;

/// Deterministic standard normal samples (`SplitMix64` + Box–Muller).
fn normal_samples(n: usize, mean: f64, sd: f64) -> Vec<f64> {
    let mut state = 1668u64;
    let mut uniform = move || {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    };
    (0..n)
        .map(|_| {
            let (u1, u2) = (1.0 - uniform(), uniform());
            let r = (-2.0 * u1.ln()).sqrt();
            mean + sd * r * (std::f64::consts::TAU * u2).cos()
        })
        .collect()
}

/// The normal density φ(x; μ, σ) as a `Log`, computed from its log so that it is exact
/// even in the far tails.
fn density(x: f64, mean: f64, sd: f64) -> Log {
    let z = (x - mean) / sd;
    Log::from_ln(-0.5 * z * z - sd.ln() - 0.5 * (std::f64::consts::TAU).ln())
}

fn main() {
    let data = normal_samples(100_000, 0.0, 1.0);

    let plain: f64 = data
        .iter()
        .map(|&x| density(x, 0.0, 1.0).to_value())
        .product();
    let true_model: Log = data.iter().map(|&x| density(x, 0.0, 1.0)).product();
    let compensated = Log::product_compensated(data.iter().map(|&x| density(x, 0.0, 1.0)));
    let shifted: Log = data.iter().map(|&x| density(x, 0.02, 1.0)).product();

    println!("likelihood of 100,000 samples under N(0, 1)");
    println!("  f64 product              {plain:e}");
    println!("  Log product              {true_model:.8}");
    println!("  Log product_compensated  {compensated:.8}");
    println!("  log-likelihood           {:.6}", true_model.ln());
    println!();
    println!("likelihood under N(0.02, 1): {shifted:.8}");

    // The ratio of two tiny numbers is an ordinary number again.
    let ratio = true_model / shifted;
    println!(
        "likelihood ratio N(0,1) : N(0.02,1) = {ratio:.4}  (in f64 range: {:.4})",
        ratio.to_value()
    );
    println!(
        "which model is more likely? {}",
        if true_model > shifted {
            "N(0, 1)"
        } else {
            "N(0.02, 1)"
        }
    );

    assert_eq!(plain, 0.0);
    assert!(true_model > Log::ZERO);
    assert!(ratio.try_to_value().is_some());
    // the plain and compensated products agree to the accuracy of the plain sum of logs
    assert!((true_model.ln() - compensated.ln()).abs() < 1e-9 * compensated.ln().abs());
}
