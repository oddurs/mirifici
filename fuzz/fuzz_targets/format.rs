//! Any stored log and any formatting flags: formatting never panics and the default
//! output parses back.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mirifici::Log;

fuzz_target!(|input: (u64, u32, u8, u8)| {
    let (bits, bits32, p, w) = input;
    let (p, w) = (usize::from(p % 64), usize::from(w));
    let ln = f64::from_bits(bits);
    if ln.is_nan() {
        return;
    }
    let x = Log::from_ln(ln);
    for s in [
        format!("{x}"),
        format!("{x:.p$}"),
        format!("{x:e}"),
        format!("{x:>w$.p$E}"),
        format!("{x:+0w$}"),
        format!("{}", x.display_base(2.0)),
    ] {
        assert!(!s.is_empty());
    }
    let _: Log = x.to_string().parse().expect("Display output must parse");
    let ln32 = f32::from_bits(bits32);
    if !ln32.is_nan() {
        let y = Log::from_ln(ln32);
        let _: Log<f32> = format!("{y:.p$}")
            .parse()
            .expect("Display output must parse");
    }
});
