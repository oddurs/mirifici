//! Any string: parsing never panics, never yields NaN, and an accepted value prints and
//! parses back to itself (outside the float range) or to within one value rounding.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mirifici::{Log, SignedLog};

fuzz_target!(|s: &str| {
    if let Ok(x) = s.parse::<Log>() {
        assert!(!x.ln().is_nan());
        let back: Log = x.to_string().parse().expect("Display output must parse");
        let tol = 2.0 * f64::EPSILON * x.ln().abs().max(1.0);
        assert!(back == x || (back.ln() - x.ln()).abs() <= tol || x.ln().abs() >= 2e16);
    }
    if let Ok(x) = s.parse::<Log<f32>>() {
        assert!(!x.ln().is_nan());
        let _: Log<f32> = x.to_string().parse().expect("Display output must parse");
    }
    if let Ok(x) = s.parse::<SignedLog>() {
        let _: SignedLog = x.to_string().parse().expect("Display output must parse");
    }
});
