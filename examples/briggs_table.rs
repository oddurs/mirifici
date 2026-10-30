//! A page of a base-10 logarithm table in the manner of Henry Briggs's *Arithmetica
//! Logarithmica* (London, 1624): fourteen decimal places, digits in groups, and the first
//! differences an interpolating reader would need.
//!
//! Briggs computed his fourteen places by hand, mostly by repeated square roots. An `f64`
//! carries 15–16 significant digits, so a five-digit number's logarithm at fourteen decimals
//! (fifteen significant digits) is right at the edge of double precision. The page is checked
//! against 25-decimal references computed with mpmath (`tests/fixtures/briggs.csv`): every
//! entry must be within 2·10⁻¹⁵ of the truth, and the program reports how many printed
//! entries differ from the correctly rounded value in the fourteenth place.
//!
//! Run with `cargo run --example briggs_table`.

use mirifici::Log;

const FIXTURE: &str = include_str!("../tests/fixtures/briggs.csv");

/// A decimal string with an integer part and at least `places` decimals, as an integer in
/// units of 10^-places (truncated).
fn to_units(s: &str, places: usize) -> i128 {
    let (int, frac) = s.split_once('.').unwrap_or((s, ""));
    let mut digits = String::from(int);
    digits.extend(frac.chars().chain(std::iter::repeat('0')).take(places));
    digits.parse().unwrap()
}

/// `units` (in 10^-14) as Briggs-style groups: `4,00004 34272 7686`.
fn briggs(units: i128) -> String {
    let int = units / 100_000_000_000_000;
    let frac = format!("{:014}", units % 100_000_000_000_000);
    format!("{int},{} {} {}", &frac[0..5], &frac[5..10], &frac[10..14])
}

/// A difference (in 10^-14) grouped in fives from the right: `4 34229 32778`.
fn grouped(units: i128) -> String {
    let s = units.to_string();
    let mut out = Vec::new();
    let mut end = s.len();
    while end > 5 {
        out.push(&s[end - 5..end]);
        end -= 5;
    }
    out.push(&s[..end]);
    out.reverse();
    out.join(" ")
}

fn main() {
    let rows: Vec<(u32, &str)> = FIXTURE
        .lines()
        .skip(1)
        .map(|l| {
            let (n, log) = l.split_once(',').unwrap();
            (n.parse().unwrap(), log)
        })
        .collect();

    // Computed entries, rounded to 14 places, and their exact errors.
    let mut entries = Vec::new();
    let mut max_err = 0i128;
    let mut mismatches = 0;
    for &(n, reference) in &rows {
        let x = Log::from_value(n as f64).log10();
        let printed = to_units(&format!("{x:.14}"), 14);
        // exact decimal expansion of the f64, to 25 places, against the reference
        let err = (to_units(&format!("{x:.25}"), 25) - to_units(reference, 25)).abs();
        max_err = max_err.max(err);
        // correctly rounded 14-place value of the reference
        let r25 = to_units(reference, 25);
        let correct = (r25 + 50_000_000_000) / 100_000_000_000;
        if printed != correct {
            mismatches += 1;
        }
        entries.push((n, printed, printed != correct));
    }

    println!();
    println!("{:^60}", "ARITHMETICA LOGARITHMICA");
    println!("{:^60}", "a page in the manner of Henry Briggs (1624)");
    println!();
    println!(
        "  {:>6}   {:<22}  {:>16}",
        "Num.", "Logarithmi.", "Differentiae."
    );
    println!("  {:-<6}   {:-<22}  {:->16}", "", "", "");
    for w in entries.windows(2).take(50) {
        let (n, log, off) = w[0];
        let diff = w[1].1 - log;
        let mark = if off { "*" } else { " " };
        println!("  {n:>6}   {:<22}{mark} {:>16}", briggs(log), grouped(diff));
        if n % 10 == 0 {
            println!();
        }
    }
    println!();
    println!(
        "max |error| of the f64 logarithms: {:.2e} (bound for this check: 2e-15)",
        max_err as f64 * 1e-25
    );
    println!(
        "entries whose 14th decimal differs from the correctly rounded value: {mismatches} of {} \
         (marked *)",
        entries.len() - 1
    );
    assert!(
        max_err < 20_000_000_000,
        "an entry is off by more than 2e-15"
    );
}
