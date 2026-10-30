#!/usr/bin/env python3
"""Generate high-precision reference fixtures for mirifici's accuracy tests.

Every fixture row holds exact floating-point inputs (written with Python's round-trip
`repr`) and the exact result computed with mpmath at 60 significant digits, split into
two doubles `ref_hi + ref_lo` so the Rust tests can measure errors well below one ulp.

Usage:  python3 tools/gen_fixtures.py        (requires `pip install mpmath`)

The output is deterministic (fixed seed). Files are written to tests/fixtures/.
"""

import csv
import math
import os
import random
import struct

import mpmath
from mpmath import mp, mpf

mp.dps = 60

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tests", "fixtures")


def to_f32(x: float) -> float:
    return struct.unpack("f", struct.pack("f", x))[0]


F32_MAX = 3.4028234663852886e38


def split(v, t=None) -> tuple[str, str]:
    """Split an mpf into hi + lo doubles (hi = nearest double).

    With `t = F32`, results beyond f32's range are written as the infinity that f32
    arithmetic overflows to.
    """
    if mpmath.isinf(v):
        return (repr(float(v)), "0.0")
    if t is F32 and abs(v) > F32_MAX:
        return (repr(math.copysign(math.inf, float(v))), "0.0")
    hi = float(v)
    if math.isinf(hi):
        return (repr(hi), "0.0")
    lo = float(v - mpf(hi))
    return (repr(hi), repr(lo))


class Type:
    def __init__(self, name, rnd, max_exp10, min_log1mexp_exp10):
        self.name = name
        self.round = rnd
        self.max_exp10 = max_exp10  # largest |ln| magnitude used, as a power of ten
        self.min_log1mexp_exp10 = min_log1mexp_exp10


F64 = Type("f64", float, 300, -300)
F32 = Type("f32", to_f32, 36, -38)


def rand_ln(rng: random.Random, t: Type) -> float:
    """A stored logarithm drawn from several scales."""
    kind = rng.randrange(4)
    if kind == 0:
        x = rng.uniform(-1, 1)
    elif kind == 1:
        x = rng.uniform(-50, 50)
    elif kind == 2:
        x = 10 ** rng.uniform(-20, 4) * rng.choice((-1, 1))
    else:
        x = 10 ** rng.uniform(4, t.max_exp10) * rng.choice((-1, 1))
    return t.round(x)


def rand_delta(rng: random.Random, t: Type) -> float:
    kind = rng.randrange(4)
    if kind == 0:
        return 0.0
    if kind == 1:
        return 10 ** rng.uniform(-18, 0)
    if kind == 2:
        return rng.uniform(0, 40)
    return 10 ** rng.uniform(1, 3)


def write(name: str, header: list[str], rows: list[list[str]]):
    path = os.path.join(OUT, name)
    with open(path, "w", newline="") as f:
        w = csv.writer(f, lineterminator="\n")
        w.writerow(header)
        w.writerows(rows)
    print(f"wrote {path} ({len(rows)} rows)")


def lse(ls):
    return mpmath.log(mpmath.fsum(mpmath.exp(mpf(l)) for l in ls))


def gen(t: Type, seed: int):
    rng = random.Random(seed)
    n = 2000

    # a + b
    rows = []
    for _ in range(n):
        a = rand_ln(rng, t)
        b = t.round(a - rng.choice((-1, 1)) * rand_delta(rng, t))
        if not (math.isfinite(a) and math.isfinite(b)):
            continue
        rows.append([repr(a), repr(b), *split(lse([a, b]))])
    write(f"add_{t.name}.csv", ["a_ln", "b_ln", "ref_hi", "ref_lo"], rows)

    # a - b for a > b
    rows = []
    for _ in range(n):
        a = rand_ln(rng, t)
        d = 10 ** rng.uniform(-6 if t is F32 else -15, 3)
        b = t.round(a - d)
        if not (b < a):
            continue
        ref = mpf(a) + mpmath.log(-mpmath.expm1(mpf(b) - mpf(a)))
        rows.append([repr(a), repr(b), *split(ref)])
    write(f"sub_{t.name}.csv", ["a_ln", "b_ln", "ref_hi", "ref_lo"], rows)

    # log1mexp(d) = ln(1 - e^-d)
    rows = []
    for i in range(n):
        if i % 4 == 0:
            d = rng.uniform(0.5, 1.0)  # around the ln 2 branch point
        else:
            d = 10 ** rng.uniform(t.min_log1mexp_exp10, 2.7)
        d = t.round(d)
        if d <= 0:
            continue
        ref = mpmath.log(-mpmath.expm1(-mpf(d)))
        rows.append([repr(d), *split(ref)])
    write(f"log1mexp_{t.name}.csv", ["d", "ref_hi", "ref_lo"], rows)

    # sums of n terms
    rows = []
    for size in (2, 3, 10, 100, 1000):
        for _ in range(40 if size < 1000 else 10):
            center = rand_ln(rng, t)
            spread = rng.choice((0.0, 1.0, 10.0, 100.0, 1000.0))
            ls = [t.round(center + rng.uniform(-spread, spread)) for _ in range(size)]
            if not all(math.isfinite(x) for x in ls):
                continue
            rows.append([";".join(repr(x) for x in ls), *split(lse(ls))])
    write(f"sum_{t.name}.csv", ["terms_ln", "ref_hi", "ref_lo"], rows)

    # a * b and a / b
    rows = []
    for _ in range(n):
        a, b = rand_ln(rng, t), rand_ln(rng, t)
        rows.append([repr(a), repr(b), *split(mpf(a) + mpf(b), t), *split(mpf(a) - mpf(b), t)])
    write(f"muldiv_{t.name}.csv", ["a_ln", "b_ln", "mul_hi", "mul_lo", "div_hi", "div_lo"], rows)

    # powf
    rows = []
    for _ in range(n):
        a = rand_ln(rng, t)
        p = t.round(rng.choice((rng.uniform(-10, 10), 10 ** rng.uniform(-10, 10) * rng.choice((-1, 1)))))
        ref = mpf(p) * mpf(a)
        rows.append([repr(a), repr(p), *split(ref, t)])
    write(f"powf_{t.name}.csv", ["a_ln", "p", "ref_hi", "ref_lo"], rows)

    # from_value: x -> ln(x), and to_value: ln -> exp(ln)
    lo_exp, hi_exp = (-307, 307) if t is F64 else (-37, 37)
    rows = []
    for _ in range(n):
        x = t.round(10 ** rng.uniform(lo_exp, hi_exp))
        rows.append([repr(x), *split(mpmath.log(mpf(x)))])
    write(f"from_value_{t.name}.csv", ["x", "ref_hi", "ref_lo"], rows)

    max_ln = 700 if t is F64 else 85
    rows = []
    for _ in range(n):
        ln = t.round(rng.uniform(-max_ln, max_ln))
        rows.append([repr(ln), *split(mpmath.exp(mpf(ln)))])
    write(f"to_value_{t.name}.csv", ["ln", "ref_hi", "ref_lo"], rows)

    # log2 / log10 accessors
    rows = []
    for _ in range(n):
        ln = rand_ln(rng, t)
        rows.append(
            [repr(ln), *split(mpf(ln) / mpmath.log(2)), *split(mpf(ln) / mpmath.log(10))]
        )
    write(f"log2_log10_{t.name}.csv", ["ln", "log2_hi", "log2_lo", "log10_hi", "log10_lo"], rows)


def sci(v, p: int) -> str:
    """`v > 0` as d.ddd(p ≥ 1 decimals)e<k>, correctly rounded from the exact value."""
    k = int(mpmath.floor(mpmath.log10(v)))
    m = v / mpf(10) ** k
    digits = mpmath.nstr(m, p + 1, strip_zeros=False, min_fixed=-1, max_fixed=2)
    if digits.startswith("10"):
        k += 1
        digits = mpmath.nstr(m / 10, p + 1, strip_zeros=False, min_fixed=-1, max_fixed=2)
    return f"{digits}e{k}"


def gen_text(t: Type, seed: int):
    """Parsing and formatting beyond the float's range."""
    rng = random.Random(seed)
    big = 10**7 if t is F64 else 10**6

    # parse: decimal strings -> ln
    rows = []
    for _ in range(1000):
        ndig = rng.randint(1, 25)
        digits = str(rng.randint(1, 9)) + "".join(str(rng.randint(0, 9)) for _ in range(ndig - 1))
        kind = rng.randrange(4)
        if kind == 0:
            k = rng.randint(-30, 30)
        elif kind == 1:
            k = rng.randint(-400, 400)
        elif kind == 2:
            k = rng.randint(-big, big)
        else:
            k = rng.choice((-1, 1)) * rng.randint(10**8, 10**15)
        point = rng.randint(1, len(digits))
        text = digits[:point] + ("." + digits[point:] if point < len(digits) else "")
        exp = k - (point - 1)
        s = f"{text}e{exp}"
        ref = mpmath.log(mpf(text) * mpf(10) ** exp)
        rows.append([s, *split(ref, t)])
    write(f"parse_{t.name}.csv", ["text", "ref_hi", "ref_lo"], rows)

    # display: stored ln (outside the float's range) -> mantissa digits at fixed precision
    p = 12 if t is F64 else 5
    lim = 745.2 if t is F64 else 103.3
    rows = []
    for _ in range(1000):
        kind = rng.randrange(3)
        if kind == 0:
            ln = rng.uniform(lim, lim * 2) * rng.choice((-1, 1))
        elif kind == 1:
            ln = rng.uniform(1e3, 1e6) * rng.choice((-1, 1))
        else:
            ln = 10 ** rng.uniform(6, 11) * rng.choice((-1, 1))
        ln = t.round(ln)
        rows.append([repr(ln), str(p), sci(mpmath.exp(mpf(ln)), p)])
    write(f"display_{t.name}.csv", ["ln", "precision", "text"], rows)


def main():
    os.makedirs(OUT, exist_ok=True)
    gen(F64, 1614)
    gen(F32, 1624)
    gen_text(F64, 1668)
    gen_text(F32, 1647)


if __name__ == "__main__":
    main()
