//! The forward algorithm for a hidden Markov model on a sequence long enough that plain
//! `f64` underflows.
//!
//! The model is the "occasionally dishonest casino" of Durbin, Eddy, Krogh and Mitchison
//! (*Biological Sequence Analysis*, 1998): a croupier usually rolls a fair die but
//! sometimes switches to one loaded towards six. Given 10,000 rolls, the forward algorithm
//! computes P(rolls) = Σ over all state paths of Π transition · emission probabilities,
//! which is about 10⁻⁷⁵⁶⁹: far below the smallest `f64`.
//!
//! Run with `cargo run --example hmm_forward`.

use mirifici::{Log, slice};

const FAIR: usize = 0;
const LOADED: usize = 1;

/// Transition probabilities `TRANS[from][to]`.
const TRANS: [[f64; 2]; 2] = [[0.95, 0.05], [0.10, 0.90]];
/// Emission probabilities for faces 1..=6.
const EMIT: [[f64; 6]; 2] = [[1.0 / 6.0; 6], [0.1, 0.1, 0.1, 0.1, 0.1, 0.5]];
const START: [f64; 2] = [0.5, 0.5];

/// A small deterministic generator (`SplitMix64`), so the example needs no dependencies.
struct Rng(u64);
impl Rng {
    fn uniform(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        (z ^ (z >> 31)) as f64 / u64::MAX as f64
    }
    fn pick(&mut self, p: &[f64]) -> usize {
        let mut u = self.uniform();
        for (i, &pi) in p.iter().enumerate() {
            if u < pi {
                return i;
            }
            u -= pi;
        }
        p.len() - 1
    }
}

fn simulate(t: usize) -> Vec<usize> {
    let mut rng = Rng(1614);
    let mut state = rng.pick(&START);
    (0..t)
        .map(|_| {
            let face = rng.pick(&EMIT[state]);
            state = rng.pick(&TRANS[state]);
            face
        })
        .collect()
}

/// Forward algorithm in plain probabilities.
fn forward_f64(rolls: &[usize]) -> f64 {
    let mut alpha = [
        START[FAIR] * EMIT[FAIR][rolls[0]],
        START[LOADED] * EMIT[LOADED][rolls[0]],
    ];
    for &r in &rolls[1..] {
        alpha = [0, 1].map(|j| (alpha[0] * TRANS[0][j] + alpha[1] * TRANS[1][j]) * EMIT[j][r]);
    }
    alpha[0] + alpha[1]
}

/// The same algorithm with `Log`: the code is unchanged apart from the type.
fn forward_log(rolls: &[usize]) -> Log {
    let p = |x: f64| Log::from_value(x);
    let mut alpha = [
        p(START[FAIR]) * p(EMIT[FAIR][rolls[0]]),
        p(START[LOADED]) * p(EMIT[LOADED][rolls[0]]),
    ];
    for &r in &rolls[1..] {
        alpha =
            [0, 1].map(|j| (alpha[0] * p(TRANS[0][j]) + alpha[1] * p(TRANS[1][j])) * p(EMIT[j][r]));
    }
    alpha[0] + alpha[1]
}

/// The textbook alternative: rescale α to sum to one at every step and add up the logs of
/// the scale factors. `slice::normalize_in_place` returns exactly that factor.
fn forward_scaled(rolls: &[usize]) -> Log {
    let p = |x: f64| Log::from_value(x);
    let mut alpha = [
        p(START[FAIR]) * p(EMIT[FAIR][rolls[0]]),
        p(START[LOADED]) * p(EMIT[LOADED][rolls[0]]),
    ];
    let mut likelihood = slice::normalize_in_place(&mut alpha).unwrap();
    for &r in &rolls[1..] {
        alpha =
            [0, 1].map(|j| (alpha[0] * p(TRANS[0][j]) + alpha[1] * p(TRANS[1][j])) * p(EMIT[j][r]));
        likelihood *= slice::normalize_in_place(&mut alpha).unwrap();
    }
    likelihood
}

fn main() {
    let rolls = simulate(10_000);
    let sixes = rolls.iter().filter(|&&r| r == 5).count();
    println!("{} rolls, {sixes} sixes", rolls.len());
    println!();

    for t in [100, 1_000, 10_000] {
        let plain = forward_f64(&rolls[..t]);
        let log = forward_log(&rolls[..t]);
        println!("first {t:>6} rolls:  f64 gives {plain:<12e}  Log gives {log:.6e}");
    }

    let log = forward_log(&rolls);
    let scaled = forward_scaled(&rolls);
    println!();
    println!("P(rolls)                 = {log:.8}");
    println!("ln P(rolls)              = {:.6}", log.ln());
    println!("as a power of two        = {:.3}", log.display_base(2.0));
    println!("per-step rescaling gives   {scaled:.8}");

    // The two orders of evaluation round differently over 10,000 steps; their stored logs
    // (about -17427, where one ulp is 3.6e-12) agree to about 1e-9.
    let rel = (log.ln() - scaled.ln()).abs();
    assert!(
        forward_f64(&rolls) == 0.0,
        "plain f64 should underflow here"
    );
    assert!(
        rel < 1e-9,
        "the two log-space computations disagree by {rel:e}"
    );
}
