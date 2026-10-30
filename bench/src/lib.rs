//! Shared inputs for the benchmarks.
//!
//! Deterministic pseudo-random data so runs are comparable.

#![allow(clippy::needless_range_loop)] // matrix code reads more clearly with indices

/// xorshift64* in [0, 1).
pub struct Rng(pub u64);

impl Rng {
    pub fn next_f64(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// `n` log-probabilities in (-20, 0].
pub fn log_probs(n: usize, seed: u64) -> Vec<f64> {
    let mut rng = Rng(seed);
    (0..n).map(|_| -20.0 * rng.next_f64()).collect()
}

/// A discrete hidden Markov model with `n` states and `m` symbols, plus an observation
/// sequence, in three storage forms: probabilities, raw logs, and `Log` values.
pub struct Hmm {
    pub n: usize,
    pub init: Vec<f64>,
    /// Row-major `trans[i * n + j]` = P(j | i).
    pub trans: Vec<f64>,
    /// Row-major `emit[j * m + o]` = P(o | j).
    pub emit: Vec<f64>,
    pub m: usize,
    pub obs: Vec<usize>,
}

impl Hmm {
    pub fn random(n: usize, m: usize, t: usize, seed: u64) -> Self {
        let mut rng = Rng(seed);
        let mut row = |k: usize| {
            let v: Vec<f64> = (0..k).map(|_| 0.1 + rng.next_f64()).collect();
            let s: f64 = v.iter().sum();
            v.into_iter().map(|x| x / s).collect::<Vec<_>>()
        };
        let init = row(n);
        let trans = (0..n).flat_map(|_| row(n)).collect();
        let emit = (0..n).flat_map(|_| row(m)).collect();
        let obs = (0..t)
            .map(|_| (rng.next_f64() * m as f64) as usize)
            .collect();
        Self {
            n,
            init,
            trans,
            emit,
            m,
            obs,
        }
    }
}

pub mod forward {
    use super::Hmm;
    use mirifici::Log;

    /// Plain probabilities. Underflows to 0 for long sequences.
    pub fn naive(h: &Hmm) -> f64 {
        let n = h.n;
        let mut a: Vec<f64> = (0..n)
            .map(|j| h.init[j] * h.emit[j * h.m + h.obs[0]])
            .collect();
        let mut next = vec![0.0; n];
        for &o in &h.obs[1..] {
            for j in 0..n {
                let mut s = 0.0;
                for i in 0..n {
                    s += a[i] * h.trans[i * n + j];
                }
                next[j] = s * h.emit[j * h.m + o];
            }
            core::mem::swap(&mut a, &mut next);
        }
        a.iter().sum::<f64>().ln()
    }

    /// Probabilities rescaled every step (Rabiner's scaling), log-likelihood accumulated.
    pub fn scaled(h: &Hmm) -> f64 {
        let n = h.n;
        let mut a: Vec<f64> = (0..n)
            .map(|j| h.init[j] * h.emit[j * h.m + h.obs[0]])
            .collect();
        let mut ll = 0.0;
        let mut next = vec![0.0; n];
        let c: f64 = a.iter().sum();
        a.iter_mut().for_each(|x| *x /= c);
        ll += c.ln();
        for &o in &h.obs[1..] {
            for j in 0..n {
                let mut s = 0.0;
                for i in 0..n {
                    s += a[i] * h.trans[i * n + j];
                }
                next[j] = s * h.emit[j * h.m + o];
            }
            let c: f64 = next.iter().sum();
            next.iter_mut().for_each(|x| *x /= c);
            ll += c.ln();
            core::mem::swap(&mut a, &mut next);
        }
        ll
    }

    /// Model parameters as raw logs, for the log-space variants.
    pub struct LogParams {
        pub init: Vec<f64>,
        pub trans: Vec<f64>,
        pub emit: Vec<f64>,
    }

    impl LogParams {
        pub fn new(h: &Hmm) -> Self {
            let ln = |v: &[f64]| v.iter().map(|x| x.ln()).collect();
            Self {
                init: ln(&h.init),
                trans: ln(&h.trans),
                emit: ln(&h.emit),
            }
        }
    }

    /// Hand-written log-space: raw f64 logs and an inline two-pass log-sum-exp.
    pub fn hand_log(h: &Hmm, p: &LogParams) -> f64 {
        let n = h.n;
        let mut a: Vec<f64> = (0..n)
            .map(|j| p.init[j] + p.emit[j * h.m + h.obs[0]])
            .collect();
        let mut next = vec![0.0; n];
        let mut tmp = vec![0.0; n];
        for &o in &h.obs[1..] {
            for j in 0..n {
                let mut m = f64::NEG_INFINITY;
                for i in 0..n {
                    tmp[i] = a[i] + p.trans[i * n + j];
                    m = m.max(tmp[i]);
                }
                let s: f64 = tmp.iter().map(|t| (t - m).exp()).sum();
                next[j] = m + s.ln() + p.emit[j * h.m + o];
            }
            core::mem::swap(&mut a, &mut next);
        }
        let m = a.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        m + a.iter().map(|x| (x - m).exp()).sum::<f64>().ln()
    }

    /// `Log<f64>` written the obvious way: `Σ_i α_i · a_ij` with the streaming `Sum`.
    pub fn log_type(h: &Hmm, init: &[Log], trans: &[Log], emit: &[Log]) -> Log {
        let n = h.n;
        let mut a: Vec<Log> = (0..n).map(|j| init[j] * emit[j * h.m + h.obs[0]]).collect();
        let mut next = vec![Log::ZERO; n];
        for &o in &h.obs[1..] {
            for (j, nj) in next.iter_mut().enumerate() {
                let s: Log = (0..n).map(|i| a[i] * trans[i * n + j]).sum();
                *nj = s * emit[j * h.m + o];
            }
            core::mem::swap(&mut a, &mut next);
        }
        a.iter().sum()
    }

    /// `Log<f64>` with a scratch buffer and `Log::sum_two_pass`.
    pub fn log_type_two_pass(h: &Hmm, init: &[Log], trans: &[Log], emit: &[Log]) -> Log {
        let n = h.n;
        let mut a: Vec<Log> = (0..n).map(|j| init[j] * emit[j * h.m + h.obs[0]]).collect();
        let mut next = vec![Log::ZERO; n];
        let mut tmp = vec![Log::ZERO; n];
        for &o in &h.obs[1..] {
            for (j, nj) in next.iter_mut().enumerate() {
                for i in 0..n {
                    tmp[i] = a[i] * trans[i * n + j];
                }
                *nj = Log::sum_two_pass(&tmp) * emit[j * h.m + o];
            }
            core::mem::swap(&mut a, &mut next);
        }
        Log::sum_two_pass(&a)
    }
}
