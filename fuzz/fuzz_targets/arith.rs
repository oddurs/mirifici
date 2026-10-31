//! Any two stored logs: no operation produces NaN, and documented panics happen only on
//! indeterminate forms.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mirifici::{Log, SignedLog};

fuzz_target!(|input: (u64, u64, u64, i32, bool, bool)| {
    let (a, b, p, n, sa, sb) = input;
    let (a, b, p) = (f64::from_bits(a), f64::from_bits(b), f64::from_bits(p));
    if a.is_nan() || b.is_nan() || p.is_nan() {
        return;
    }
    let (x, y) = (Log::from_ln(a), Log::from_ln(b));
    let ok = |v: Log| assert!(!v.ln().is_nan());
    ok(x + y);
    for r in [x.checked_mul(y), x.checked_div(y), x.checked_sub(y)]
        .into_iter()
        .flatten()
    {
        ok(r);
    }
    ok(x.powf(p));
    ok(x.powi(n));
    ok(x.sqrt());
    ok(x.recip());
    ok([x, y, x].iter().sum());
    ok(Log::sum_two_pass(&[x, y]));
    let (u, v) = (SignedLog::new(sa, x), SignedLog::new(sb, y));
    for r in [
        u.checked_add(v),
        u.checked_sub(v),
        u.checked_mul(v),
        u.checked_div(v),
    ]
    .into_iter()
    .flatten()
    {
        ok(r.magnitude());
    }
});
