//! With the `libm` feature, results are bit-for-bit identical on every platform.
//!
//! The test runs a fixed battery of operations, folds the exact bits of every result (and
//! every formatted string) into one hash, and compares it with a constant. CI runs it on
//! Linux x86-64, macOS arm64 and Windows; a change in the constant means a change in
//! results, which must be deliberate and noted in the changelog.
#![cfg(feature = "libm")]

use mirifici::{Log, SignedLog, log1mexp, slice};

/// The expected hash. Update only with a changelog entry explaining why results changed.
const EXPECTED_F64: u64 = 0x8ad0874bff970234;
const EXPECTED_F32: u64 = 0x63bcfb386e29169f;

/// FNV-1a, so the test needs no dependencies.
struct Hash(u64);
impl Hash {
    fn bytes(&mut self, b: &[u8]) {
        for &x in b {
            self.0 = (self.0 ^ u64::from(x)).wrapping_mul(0x0100_0000_01b3);
        }
    }
    fn f64(&mut self, x: f64) {
        self.bytes(&x.to_bits().to_le_bytes());
    }
    fn f32(&mut self, x: f32) {
        self.bytes(&x.to_bits().to_le_bytes());
    }
    fn str(&mut self, s: &str) {
        self.bytes(s.as_bytes());
        self.bytes(&[0]);
    }
}

/// `SplitMix64`: deterministic inputs spanning many scales.
struct Inputs(u64);
impl Inputs {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// A stored log between −10^6 and 10^6, log-uniformly in magnitude.
    fn ln(&mut self) -> f64 {
        // Through the crate (libm), not `f64::powf`, which is the platform's.
        let mag = Log::<f64>::from_log10(self.unit() * 12.0 - 6.0).to_value();
        if self.next_u64() & 1 == 0 { mag } else { -mag }
    }
}

fn battery_f64() -> u64 {
    let mut h = Hash(0xcbf2_9ce4_8422_2325);
    let mut r = Inputs(1614);
    let mut xs = Vec::new();
    for _ in 0..2000 {
        let (a, b) = (Log::<f64>::from_ln(r.ln()), Log::<f64>::from_ln(r.ln()));
        let v = r.unit() * 1e3;
        for x in [
            a + b,
            a.checked_sub(b).unwrap_or(Log::ZERO),
            b.abs_diff(a),
            a * b,
            a / b,
            a.powf(r.unit() * 7.0 - 3.0),
            a.sqrt(),
            Log::from_value(v),
            Log::from_log10(a.ln()),
            Log::from_log2(b.ln()),
        ] {
            h.f64(x.ln());
        }
        h.f64(a.log2());
        h.f64(a.log10());
        h.f64(Log::<f64>::from_ln(r.unit() * 1400.0 - 700.0).to_value());
        h.f64(log1mexp(r.unit() * 3.0));
        h.str(&a.to_string());
        h.str(&format!("{a:.6e}"));
        h.str(&format!("{}", b.display_base(2.0)));
        let s = a.to_string();
        h.f64(s.parse::<Log>().unwrap().ln());
        let sa = SignedLog::new(r.next_u64() & 1 == 0, a);
        let sb = SignedLog::new(r.next_u64() & 1 == 0, b);
        h.f64((sa + sb).magnitude().ln());
        h.f64((sa - sb).magnitude().ln());
        xs.push(a);
    }
    h.f64(xs.iter().sum::<Log>().ln());
    h.f64(Log::sum_two_pass(&xs).ln());
    h.f64(xs.iter().product::<Log>().ln());
    h.f64(Log::product_compensated(xs.iter().copied()).ln());
    let mut raw: Vec<f64> = xs.iter().take(100).map(|x| x.ln() * 1e-3).collect();
    h.f64(slice::logsumexp(&raw));
    slice::softmax_in_place(&mut raw);
    for &x in &raw {
        h.f64(x);
    }
    h.0
}

fn battery_f32() -> u64 {
    let mut h = Hash(0xcbf2_9ce4_8422_2325);
    let mut r = Inputs(1620);
    for _ in 0..2000 {
        let a = Log::<f32>::from_ln((r.ln() * 1e-2) as f32);
        let b = Log::<f32>::from_ln((r.ln() * 1e-2) as f32);
        for x in [
            a + b,
            a.checked_sub(b).unwrap_or(Log::ZERO),
            a * b,
            a.powf(1.5),
        ] {
            h.f32(x.ln());
        }
        h.f32(log1mexp((r.unit() * 3.0) as f32));
        h.f32(Log::<f32>::from_value((r.unit() * 100.0) as f32).ln());
        h.str(&a.to_string());
    }
    h.0
}

#[test]
fn results_are_identical_on_every_platform() {
    let (f64_hash, f32_hash) = (battery_f64(), battery_f32());
    println!("f64: {f64_hash:#018x}  f32: {f32_hash:#018x}");
    assert_eq!(
        f64_hash, EXPECTED_F64,
        "f64 results changed (got {f64_hash:#018x})"
    );
    assert_eq!(
        f32_hash, EXPECTED_F32,
        "f32 results changed (got {f32_hash:#018x})"
    );
}
