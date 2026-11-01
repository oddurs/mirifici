//! The website's calculator: `mirifici` compiled to WebAssembly.
//!
//! A small expression language evaluated with `SignedLog<f64>`, so values far outside the
//! range of `f64` (and negative ones) are computed with the crate's documented accuracy.
//! Every value carries a running bound on the absolute error of its stored logarithm (which
//! is its relative error), propagated with the rules of `docs/accuracy.md`; the result is
//! shown with only the digits that bound justifies.
//!
//! ```text
//! expr    = term   { ("+" | "-") term }
//! term    = unary  { ("*" | "/" | "×" | "·" | "÷") unary }
//! unary   = "-" unary | power
//! power   = postfix [ "^" unary ]            (right-associative)
//! postfix = primary { "!" }
//! primary = number | "(" expr ")" | name "(" expr ")" | constant
//! ```
//!
//! The JavaScript side calls [`alloc`], copies UTF-8 in, calls [`evaluate`], reads the JSON
//! result, and frees both buffers with [`dealloc`].

use mirifici::{Log, SignedLog};

type S = SignedLog<f64>;

const EPS: f64 = f64::EPSILON;

/// The largest n accepted by `n!` (the product takes a few milliseconds).
const MAX_FACTORIAL: f64 = 10_000_000.0;

/// A value and a bound on the absolute error of its stored log (= its relative error).
#[derive(Clone, Copy, Debug)]
pub struct V {
    /// The value.
    pub x: S,
    /// Bound on |computed ln|x| − exact ln|x||.
    pub err: f64,
}

/// One rounding of a stored log of magnitude |ln x|: ε·max(1, |ln x|).
fn rounding(x: S) -> f64 {
    let l = x.magnitude().ln().abs();
    if l.is_finite() { EPS * l.max(1.0) } else { 0.0 }
}

impl V {
    /// A freshly rounded value (a literal, or the result of one correctly rounded step).
    fn rounded(x: S) -> V {
        V {
            x,
            err: rounding(x),
        }
    }
    fn with(x: S, inherited: f64) -> V {
        V {
            x,
            err: inherited + rounding(x),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(String),
    Name(String),
    Op(char),
}

fn tokenize(s: &str) -> Result<Vec<Tok>, String> {
    let cs: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || c == '.' {
            let start = i;
            while i < cs.len() && (cs[i].is_ascii_digit() || cs[i] == '.' || cs[i] == '_') {
                i += 1;
            }
            // exponent: e/E, optional sign, at least one digit
            if i < cs.len() && (cs[i] == 'e' || cs[i] == 'E') {
                let mut j = i + 1;
                if j < cs.len() && (cs[j] == '+' || cs[j] == '-' || cs[j] == '−') {
                    j += 1;
                }
                if j < cs.len() && cs[j].is_ascii_digit() {
                    while j < cs.len() && (cs[j].is_ascii_digit() || cs[j] == '_') {
                        j += 1;
                    }
                    i = j;
                }
            }
            let text: String = cs[start..i]
                .iter()
                .filter(|&&c| c != '_')
                .map(|&c| if c == '−' { '-' } else { c })
                .collect();
            out.push(Tok::Num(text));
        } else if c.is_alphabetic() {
            let start = i;
            while i < cs.len() && (cs[i].is_alphanumeric() || cs[i] == '_') {
                i += 1;
            }
            out.push(Tok::Name(
                cs[start..i].iter().collect::<String>().to_lowercase(),
            ));
        } else {
            let op = match c {
                '+' | '-' | '*' | '/' | '^' | '!' | '(' | ')' => c,
                '−' => '-',
                '×' | '·' => '*',
                '÷' => '/',
                '∞' => {
                    out.push(Tok::Name("inf".into()));
                    i += 1;
                    continue;
                }
                _ => return Err(format!("unexpected character '{c}'")),
            };
            out.push(Tok::Op(op));
            i += 1;
        }
    }
    Ok(out)
}

/// a + b with the error of each input weighted by its share: for r = a + b,
/// d ln|r| = (a/r) d ln|a| + (b/r) d ln|b|, which amplifies errors when a and b cancel.
fn add(a: V, b: V) -> Result<V, String> {
    let r = a.x.checked_add(b.x).ok_or("∞ − ∞ is indeterminate")?;
    if r == S::ZERO || r.magnitude() == Log::INFINITY {
        return Ok(V { x: r, err: 0.0 });
    }
    let lr = r.magnitude().ln();
    let share = |v: V| {
        if v.x == S::ZERO {
            0.0
        } else {
            (v.x.magnitude().ln() - lr).exp() * v.err
        }
    };
    // Documented bounds: addition ε·max(1, |ln r|); subtraction 2ε·max(1, |la|, |ln r|).
    let big = a.x.magnitude().ln().abs().max(b.x.magnitude().ln().abs());
    let own = if a.x.is_negative() == b.x.is_negative() {
        rounding(r)
    } else {
        2.0 * EPS * big.max(lr.abs()).max(1.0)
    };
    Ok(V {
        x: r,
        err: share(a) + share(b) + own,
    })
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn eat(&mut self, op: char) -> bool {
        if self.peek() == Some(&Tok::Op(op)) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expr(&mut self) -> Result<V, String> {
        let mut x = self.term()?;
        loop {
            if self.eat('+') {
                let y = self.term()?;
                x = add(x, y)?;
            } else if self.eat('-') {
                let y = self.term()?;
                x = add(
                    x,
                    V {
                        x: -y.x,
                        err: y.err,
                    },
                )?;
            } else {
                return Ok(x);
            }
        }
    }

    fn term(&mut self) -> Result<V, String> {
        let mut x = self.unary()?;
        loop {
            if self.eat('*') {
                let y = self.unary()?;
                let r = x.x.checked_mul(y.x).ok_or("0 · ∞ is indeterminate")?;
                x = V::with(r, x.err + y.err);
            } else if self.eat('/') {
                let y = self.unary()?;
                let r =
                    x.x.checked_div(y.x)
                        .ok_or("0/0 and ∞/∞ are indeterminate")?;
                x = V::with(r, x.err + y.err);
            } else {
                return Ok(x);
            }
        }
    }

    fn unary(&mut self) -> Result<V, String> {
        if self.eat('-') {
            let v = self.unary()?;
            return Ok(V {
                x: -v.x,
                err: v.err,
            });
        }
        if self.eat('+') {
            return self.unary();
        }
        self.power()
    }

    fn power(&mut self) -> Result<V, String> {
        let base = self.postfix()?;
        if !self.eat('^') {
            return Ok(base);
        }
        let exp = self.unary()?;
        pow(base, exp)
    }

    fn postfix(&mut self) -> Result<V, String> {
        let mut x = self.primary()?;
        while self.eat('!') {
            x = factorial(x)?;
        }
        Ok(x)
    }

    fn primary(&mut self) -> Result<V, String> {
        match self.toks.get(self.pos).cloned() {
            Some(Tok::Num(t)) => {
                self.pos += 1;
                let x: S = t.parse().map_err(|e| format!("'{t}': {e}"))?;
                Ok(V::rounded(x))
            }
            Some(Tok::Op('(')) => {
                self.pos += 1;
                let x = self.expr()?;
                if !self.eat(')') {
                    return Err("missing ')'".into());
                }
                Ok(x)
            }
            Some(Tok::Name(n)) => {
                self.pos += 1;
                match n.as_str() {
                    "e" => return Ok(V::rounded(S::from_value(core::f64::consts::E))),
                    "pi" | "π" => return Ok(V::rounded(S::from_value(core::f64::consts::PI))),
                    "inf" | "infinity" => {
                        return Ok(V {
                            x: S::INFINITY,
                            err: 0.0,
                        });
                    }
                    _ => {}
                }
                if !self.eat('(') {
                    return Err(format!("unknown name '{n}'"));
                }
                let x = self.expr()?;
                if !self.eat(')') {
                    return Err("missing ')'".into());
                }
                function(&n, x)
            }
            Some(Tok::Op(c)) => Err(format!("unexpected '{c}'")),
            None => Err("expression ends early".into()),
        }
    }
}

/// The value of `x` as an `f64`, for exponents and function arguments.
fn finite(x: S, what: &str) -> Result<f64, String> {
    let v = x.to_value();
    if v.is_finite() || x.magnitude() == Log::INFINITY {
        Ok(v)
    } else {
        Err(format!("{what} is outside the range of f64"))
    }
}

/// `v` snapped to the nearest integer if it is one up to its error bound (a value that went
/// through log space, like `3`, comes back as 3.0000000000000004).
fn as_integer(v: f64, err: f64) -> Option<f64> {
    let r = v.round();
    ((v - r).abs() <= 2.0 * err.max(EPS) * v.abs().max(1.0)).then_some(r)
}

/// xᵖ = exp(p·ln x): errors scale by |p| (base) and by |p·ln x| (exponent).
fn pow(base: V, exp: V) -> Result<V, String> {
    let p = finite(exp.x, "the exponent")?;
    if p.is_infinite() {
        return Err("infinite exponents are not supported".into());
    }
    let snapped = as_integer(p, exp.err);
    let p = snapped.unwrap_or(p);
    if base.x.is_negative() && snapped.is_none() {
        return Err("a negative number to a fractional power is not real".into());
    }
    let negative = base.x.is_negative() && (p / 2.0).fract() != 0.0;
    let r = S::new(negative, base.x.magnitude().powf(p));
    let lb = base.x.magnitude().ln().abs();
    let from_exp = if snapped.is_some() || !lb.is_finite() {
        0.0
    } else {
        exp.err * (p * lb).abs()
    };
    Ok(V::with(r, p.abs() * base.err + from_exp))
}

fn factorial(x: V) -> Result<V, String> {
    let v = finite(x.x, "the factorial's argument")?;
    let n = match as_integer(v, x.err) {
        Some(n) if !x.x.is_negative() => n,
        _ => return Err("n! needs a non-negative integer".into()),
    };
    if n > MAX_FACTORIAL {
        return Err(format!("n! is limited to n ≤ {MAX_FACTORIAL:e} here"));
    }
    let n = n as u32;
    let product = Log::product_compensated((2..=n).map(|k| Log::from_value(f64::from(k))));
    // Each ln k is rounded once (at most ½ε·ln n); the compensated sum adds about one ulp.
    let bound = 0.5 * EPS * f64::from(n) * f64::from(n.max(2)).ln();
    Ok(V::with(S::from(product), bound))
}

fn positive(x: V, f: &str) -> Result<Log<f64>, String> {
    if x.x.is_negative() || x.x == S::ZERO {
        Err(format!("{f} needs a positive argument"))
    } else {
        Ok(x.x.magnitude())
    }
}

/// A plain number `v` (a logarithm, say) known to an absolute error `abs_err`.
fn number(v: f64, abs_err: f64) -> V {
    let x = S::from_value(v);
    V::with(x, if v == 0.0 { 0.0 } else { abs_err / v.abs() })
}

fn function(name: &str, x: V) -> Result<V, String> {
    Ok(match name {
        // ln x is the stored log itself, known to ±err (absolute).
        "ln" => number(positive(x, "ln")?.ln(), x.err),
        "log10" | "log" => number(
            positive(x, "log10")?.log10(),
            x.err * core::f64::consts::LOG10_E,
        ),
        "log2" => number(
            positive(x, "log2")?.log2(),
            x.err * core::f64::consts::LOG2_E,
        ),
        "sqrt" => {
            if x.x.is_negative() {
                return Err("sqrt needs a non-negative argument".into());
            }
            V {
                x: x.x.magnitude().sqrt().into(),
                err: x.err / 2.0,
            }
        }
        "exp" => {
            let v = finite(x.x, "exp's argument")?;
            // exp(v) stores v, whose absolute error is |v|·err.
            V::with(Log::from_ln(v).into(), v.abs() * x.err)
        }
        "abs" => V {
            x: x.x.magnitude().into(),
            err: x.err,
        },
        _ => return Err(format!("unknown function '{name}'")),
    })
}

/// Evaluates an expression.
///
/// # Errors
///
/// On syntax errors, indeterminate forms and out-of-domain arguments.
pub fn eval(src: &str) -> Result<V, String> {
    let toks = tokenize(src)?;
    if toks.is_empty() {
        return Err("type an expression".into());
    }
    let mut p = Parser { toks, pos: 0 };
    let x = p.expr()?;
    if p.pos != p.toks.len() {
        return Err("unexpected input after the expression".into());
    }
    Ok(x)
}

fn json_str(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// Significant digits justified by a relative error bound.
#[must_use]
pub fn digits(err: f64) -> usize {
    ((-err.max(EPS).log10()).floor() as usize).clamp(1, 15)
}

/// `v.x` rounded to the digits its error bound justifies, in positional notation when short.
#[must_use]
pub fn pretty(v: V) -> String {
    let (x, m) = (v.x, v.x.magnitude());
    if m == Log::ZERO {
        return "0".into();
    }
    if m == Log::INFINITY {
        return if x.is_negative() {
            "−∞".into()
        } else {
            "∞".into()
        };
    }
    let s = format!("{:.*e}", digits(v.err) - 1, x);
    let (mant, exp) = s.split_once('e').unwrap_or((&s, "0"));
    let mant = if mant.contains('.') {
        mant.trim_end_matches('0').trim_end_matches('.')
    } else {
        mant
    };
    let a = x.to_value().abs();
    if m.try_to_value().is_some() && (1e-6..1e15).contains(&a) {
        // Short enough for positional notation; reparse the rounded digits.
        let r: f64 = format!("{mant}e{exp}").parse().unwrap_or(f64::NAN);
        return format!("{r}").replace('-', "−");
    }
    format!("{mant}e{exp}").replace('-', "−")
}

/// The JSON the page shows for an input.
#[must_use]
pub fn evaluate_json(src: &str) -> String {
    match eval(src) {
        Err(e) => format!("{{\"ok\":false,\"error\":{}}}", json_str(&e)),
        Ok(v) => {
            let (x, m) = (v.x, v.x.magnitude());
            let sign = if x.is_negative() { "−" } else { "" };
            let fields = [
                ("value", pretty(v)),
                ("exact", format!("{x:e}").replace('-', "−")),
                ("ln", format!("{:?}", m.ln()).replace('-', "−")),
                ("err", format!("{:.1e}", v.err).replace('-', "−")),
                ("digits", digits(v.err).to_string()),
                (
                    "pow2",
                    format!("{sign}{:.6}", m.display_base(2.0)).replace('-', "−"),
                ),
                (
                    "pow10",
                    format!("{sign}{:.6}", m.display_base(10.0)).replace('-', "−"),
                ),
                ("f64", format!("{:e}", x.to_value()).replace('-', "−")),
            ];
            let body: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("\"{k}\":{}", json_str(v)))
                .collect();
            format!("{{\"ok\":true,{}}}", body.join(","))
        }
    }
}

// ---- WebAssembly interface -----------------------------------------------------------------

/// Allocates `len` bytes for the caller to write UTF-8 into.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(len);
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

/// Frees a buffer returned by [`alloc`] or [`evaluate`].
///
/// # Safety
///
/// `ptr` and `cap` must come from one of those calls, and the buffer must not be used again.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, cap: usize) {
    drop(unsafe { Vec::from_raw_parts(ptr, 0, cap) });
}

/// Evaluates the UTF-8 expression at `ptr..ptr+len` and returns the JSON result as
/// `(pointer << 32) | length`; free it with `dealloc(pointer, length)`.
///
/// # Safety
///
/// `ptr..ptr+len` must be a readable buffer from [`alloc`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn evaluate(ptr: *const u8, len: usize) -> u64 {
    let input = unsafe { core::slice::from_raw_parts(ptr, len) };
    let out = match core::str::from_utf8(input) {
        Ok(s) => evaluate_json(s),
        Err(_) => "{\"ok\":false,\"error\":\"input is not UTF-8\"}".to_owned(),
    };
    let boxed = out.into_bytes().into_boxed_slice();
    let n = boxed.len();
    let p = Box::into_raw(boxed).cast::<u8>();
    ((p as usize as u64) << 32) | n as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn show(s: &str) -> String {
        match eval(s) {
            Ok(v) => pretty(v).replace('−', "-"),
            Err(e) => format!("error: {e}"),
        }
    }

    #[test]
    fn arithmetic() {
        assert_eq!(show("1e-5000 * 1e-5000"), "1e-10000");
        assert_eq!(show("2.5e-90210 + 2.5e-90210"), "5e-90210");
        assert_eq!(show("1e400 - 1e400 + 1"), "1");
        assert_eq!(show("2 - 5"), "-3");
        assert_eq!(show("-2^2"), "-4");
        assert_eq!(show("(-2)^3"), "-8");
        assert_eq!(show("2^3^2"), "512");
        assert_eq!(show("1e-300^1000"), "1e-300000");
        assert_eq!(show("2 × 3 ÷ 4 − 1"), "0.5");
        assert_eq!(show("0.1 + 0.2"), "0.3");
    }

    #[test]
    fn error_bounds_follow_the_logs() {
        // Each factorial's stored log is ~1e6, so the quotient keeps ~10 digits.
        let v = eval("100000! / 99999!").unwrap();
        assert!(v.err > 1e-11 && v.err < 1e-9, "{}", v.err);
        assert_eq!(pretty(v), "100000");
        // Catastrophic cancellation shows up as a large bound.
        let c = eval("(1 + 1e-12) - 1").unwrap();
        assert!(c.err > 1e-6, "{}", c.err);
        assert!(eval("2 * 3").unwrap().err < 1e-15);
    }

    #[test]
    fn factorials_and_functions() {
        assert_eq!(show("5!"), "120");
        assert_eq!(show("0!"), "1");
        // ln(10⁶!) ≈ 1.3e7, so the stored log supports about 8 significant digits.
        assert!(
            show("1000000!").starts_with("8.26393"),
            "{}",
            show("1000000!")
        );
        assert!(
            show("1000000!").ends_with("e5565708"),
            "{}",
            show("1000000!")
        );
        assert_eq!(show("sqrt(1e-99999 * 10)"), "1e-49999");
        assert_eq!(show("log10(1e-5000)"), "-5000");
        // 3.03321539680…e434294, to the 8 digits the bound on ln = 1e6 allows
        assert_eq!(show("exp(1e6)"), "3.0332154e434294");
    }

    #[test]
    fn errors() {
        assert!(show("0 * inf").contains("indeterminate"));
        assert!(show("inf - inf").contains("indeterminate"));
        assert!(show("(-8)^0.5").contains("not real"));
        assert!(show("ln(-1)").contains("positive"));
        assert!(show("2.5!").contains("integer"));
        assert!(show("1 +").contains("ends early"));
        assert!(show("foo(1)").contains("unknown"));
        assert!(show("(1").contains("')'"));
        assert!(show("1 2").contains("after"));
        assert!(show("1 $ 2").contains("unexpected character"));
    }

    #[test]
    fn json() {
        let j = evaluate_json("1e-5000");
        assert!(j.contains("\"value\":\"1e−5000\""), "{j}");
        assert!(j.contains("\"f64\":\"0e0\""), "{j}");
        assert!(evaluate_json("\"").contains("\"ok\":false"));
    }
}
