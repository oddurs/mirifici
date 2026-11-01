// mirifici site: KaTeX, theme, and the interactive figures. No dependencies beyond KaTeX.
"use strict";

const SVGNS = "http://www.w3.org/2000/svg";
const LOG10_2 = Math.log10(2);

/** Creates an SVG element with attributes, appended to `parent` if given. */
function el(tag, attrs = {}, parent) {
  const e = document.createElementNS(SVGNS, tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
  if (parent) parent.appendChild(e);
  return e;
}

function text(parent, x, y, str, cls, anchor = "start", baseline = "auto") {
  const t = el("text", { x, y, class: cls, "text-anchor": anchor, "dominant-baseline": baseline }, parent);
  t.textContent = str;
  return t;
}

/** Linear scale. */
function scale(d0, d1, r0, r1) {
  const f = (v) => r0 + ((v - d0) / (d1 - d0)) * (r1 - r0);
  f.invert = (p) => d0 + ((p - r0) / (r1 - r0)) * (d1 - d0);
  return f;
}

/** "Nice" tick values covering [lo, hi]. */
function ticks(lo, hi, count = 5) {
  const span = hi - lo;
  const step0 = Math.pow(10, Math.floor(Math.log10(span / count)));
  const err = (count * step0) / span;
  const step = step0 * (err <= 0.15 ? 10 : err <= 0.35 ? 5 : err <= 0.75 ? 2 : 1);
  const out = [];
  for (let v = Math.ceil(lo / step) * step; v <= hi + step * 1e-9; v += step) out.push(+v.toPrecision(12));
  return out;
}

const fmt = new Intl.NumberFormat("en-US");
const minus = (s) => String(s).replace(/-/g, "−");

/** Scientific notation from a base-10 logarithm, as mirifici's Display does beyond range. */
function sciFromLog10(l10, digits = 4) {
  if (l10 === -Infinity) return "0";
  let e = Math.floor(l10);
  let m = Math.pow(10, l10 - e);
  let ms = m.toFixed(digits);
  if (ms.startsWith("10")) { e += 1; ms = (1).toFixed(digits); }
  return `${ms}e${e}`;
}

// ---- tooltip --------------------------------------------------------------------------
const tip = document.querySelector(".tip");
function showTip(html, x, y) {
  tip.innerHTML = html;
  tip.hidden = false;
  const r = tip.getBoundingClientRect();
  let left = x + 14, top = y + 14;
  if (left + r.width > innerWidth - 8) left = x - r.width - 14;
  if (top + r.height > innerHeight - 8) top = y - r.height - 14;
  tip.style.left = `${Math.max(8, left)}px`;
  tip.style.top = `${Math.max(8, top)}px`;
}
function hideTip() { tip.hidden = true; }

// ---- KaTeX ----------------------------------------------------------------------------
function tex(node, src, display = false) {
  if (!window.katex) { node.textContent = src; return; }
  try { katex.render(src, node, { displayMode: display, throwOnError: false, output: "html" }); }
  catch (_) { node.textContent = src; }
}
function renderMath() {
  for (const n of document.querySelectorAll("[data-tex]")) {
    tex(n, n.dataset.tex, n.classList.contains("display") || n.classList.contains("hero-eq"));
  }
}

// ---- theme ----------------------------------------------------------------------------
function initTheme() {
  const btn = document.querySelector(".theme");
  btn.addEventListener("click", () => {
    const root = document.documentElement;
    const dark = root.dataset.theme
      ? root.dataset.theme === "dark"
      : matchMedia("(prefers-color-scheme: dark)").matches;
    root.dataset.theme = dark ? "light" : "dark";
    try { localStorage.setItem("theme", root.dataset.theme); } catch (_) {}
  });
}

// ---- hero curve -----------------------------------------------------------------------
function drawHero() {
  const W = 1200, H = 420;
  const x = scale(0.18, 7.2, 0, W), y = scale(0, 5.2, H, 0);
  let line = "", area = `M${x(1)},${y(0)}`;
  for (let i = 0; i <= 400; i++) {
    const t = 0.2 + (i / 400) * 7;
    line += `${i ? "L" : "M"}${x(t).toFixed(1)},${y(1 / t).toFixed(1)}`;
    if (t >= 1 && t <= Math.E) area += `L${x(t).toFixed(1)},${y(1 / t).toFixed(1)}`;
  }
  area += `L${x(Math.E)},${y(1 / Math.E)}L${x(Math.E)},${y(0)}Z`;
  document.getElementById("hero-line").setAttribute("d", line);
  document.getElementById("hero-area").setAttribute("d", area);
}

// ---- I. the hyperbola -----------------------------------------------------------------
const hyp = { x: 1.8 };
function drawHyperbola() {
  const svg = document.getElementById("hyperbola");
  const W = Math.max(300, svg.clientWidth || 520);
  const H = Math.round(Math.min(400, Math.max(260, W * 0.68)));
  svg.setAttribute("viewBox", `0 0 ${W} ${H}`);
  svg.replaceChildren();
  const m = { l: 34, r: 14, t: 16, b: 30 };
  const tx = scale(0, 7.2, m.l, W - m.r), ty = scale(0, 1.75, H - m.b, m.t);
  const X = hyp.x, X2 = X * X;

  for (const v of [0.5, 1, 1.5]) {
    el("line", { x1: m.l, x2: W - m.r, y1: ty(v), y2: ty(v), class: "gridline" }, svg);
    text(svg, m.l - 6, ty(v), String(v), "tick-label", "end", "middle");
  }
  // integer ticks, except where the x and x² labels sit
  for (let v = 1; v <= 7; v++) {
    if (Math.abs(v - hyp.x) < 0.3 || Math.abs(v - hyp.x * hyp.x) < 0.35) continue;
    text(svg, tx(v), H - m.b + 18, String(v), "tick-label", "middle");
  }

  const region = (a, b, cls) => {
    let d = `M${tx(a)},${ty(0)}`;
    for (let i = 0; i <= 80; i++) { const t = a + ((b - a) * i) / 80; d += `L${tx(t)},${ty(1 / t)}`; }
    el("path", { d: d + `L${tx(b)},${ty(0)}Z`, class: cls }, svg);
  };
  region(1, X, "area-1");
  region(X, X2, "area-2");

  let d = "";
  for (let i = 0; i <= 300; i++) { const t = 0.58 + (i / 300) * 6.6; d += `${i ? "L" : "M"}${tx(t)},${ty(1 / t)}`; }
  el("path", { d, class: "curve" }, svg);
  el("line", { x1: m.l, x2: W - m.r, y1: ty(0), y2: ty(0), class: "axis" }, svg);
  el("line", { x1: m.l, x2: m.l, y1: ty(0), y2: m.t, class: "axis" }, svg);
  text(svg, tx(0.66), ty(1 / 0.66) + 4, "y = 1/t", "annot", "start", "hanging").setAttribute("dx", "10");

  // region labels
  const lab = (a, b, s) => text(svg, tx((a + b) / 2), ty(0.5 / Math.max(a, (a + b) / 2)) , s, "annot strong", "middle", "middle");
  lab(1, X, "ln x");
  lab(X, X2, "ln x");
  text(svg, tx(X), H - m.b + 18, "x", "annot strong", "middle");
  text(svg, tx(X2), H - m.b + 18, "x²", "annot strong", "middle");

  // draggable handle on the curve at t = x
  el("circle", { cx: tx(X), cy: ty(1 / X), r: 9, class: "handle" }, svg);
  hyp.toT = (clientX) => { const r = svg.getBoundingClientRect(); return tx.invert(((clientX - r.left) / r.width) * W); };
  svg.setAttribute("aria-valuenow", X.toFixed(2));

  const ln = Math.log(X).toFixed(4);
  tex(document.getElementById("hyp-readout"),
    `\\int_1^{${X.toFixed(2)}}\\frac{dt}{t} = ${ln} = \\int_{${X.toFixed(2)}}^{${X2.toFixed(2)}}\\frac{dt}{t}`);
}

// Pointer and keyboard handling live on the <svg>, which survives redraws (its children don't).
function initHyperbola() {
  const svg = document.getElementById("hyperbola");
  Object.entries({ tabindex: 0, role: "slider", "aria-label": "x", "aria-valuemin": 1.05, "aria-valuemax": 2.6 })
    .forEach(([k, v]) => svg.setAttribute(k, v));
  const set = (v) => { hyp.x = Math.min(2.6, Math.max(1.05, v)); document.getElementById("hyp-x").value = hyp.x; drawHyperbola(); };
  let dragging = false;
  svg.addEventListener("pointerdown", (e) => { dragging = true; svg.setPointerCapture(e.pointerId); set(hyp.toT(e.clientX)); });
  svg.addEventListener("pointermove", (e) => { if (dragging) set(hyp.toT(e.clientX)); });
  const stop = () => { dragging = false; };
  svg.addEventListener("pointerup", stop);
  svg.addEventListener("pointercancel", stop);
  svg.addEventListener("keydown", (e) => {
    if (e.key === "ArrowRight" || e.key === "ArrowUp") { set(hyp.x + 0.02); e.preventDefault(); }
    if (e.key === "ArrowLeft" || e.key === "ArrowDown") { set(hyp.x - 0.02); e.preventDefault(); }
  });
}

// ---- II. underflow --------------------------------------------------------------------
const uf = { p: 0.4, n: 1995 };
function readUnderflow() {
  uf.p = +document.getElementById("uf-p").value;
  uf.n = Math.max(1, Math.round(Math.pow(10, +document.getElementById("uf-n").value)));
  document.getElementById("uf-p-out").textContent = uf.p.toFixed(2);
  document.getElementById("uf-n-out").textContent = fmt.format(uf.n);
}

/** Smallest n for which p^n is 0 in f64. */
function underflowN(p) {
  let n = Math.floor(-323.3 / Math.log10(p)) - 3;
  while (Math.pow(p, n) > 0) n++;
  return n;
}

function drawUnderflow() {
  const { p, n } = uf;
  const f = Math.pow(p, n);
  const l10 = n * Math.log10(p);
  const n0 = underflowN(p);
  document.getElementById("uf-float").textContent = f === 0 ? "0" : minus(f.toExponential(4));
  document.getElementById("uf-float-note").textContent =
    f === 0 ? `Underflowed at n = ${fmt.format(n0)}. The information is gone.`
      : f < 2.2250738585072014e-308 ? "Subnormal: most significant bits already lost."
        : "Still in range.";
  document.getElementById("uf-log").textContent = minus(sciFromLog10(l10));
  document.getElementById("uf-log-note").textContent =
    `Stored log: n · ln p = ${minus((n * Math.log(p)).toPrecision(8))}`;

  const svg = document.getElementById("uf-chart");
  const W = Math.max(320, svg.clientWidth || 900), H = Math.round(Math.max(220, Math.min(320, W * 0.34)));
  svg.setAttribute("viewBox", `0 0 ${W} ${H}`);
  svg.replaceChildren();
  const m = { l: 62, r: 18, t: 14, b: 34 };
  const yMin = Math.min(l10, -10) * 1.04;
  const sx = scale(0, n, m.l, W - m.r), sy = scale(yMin, 0, H - m.b, m.t);

  for (const v of ticks(yMin, 0, 5)) {
    el("line", { x1: m.l, x2: W - m.r, y1: sy(v), y2: sy(v), class: "gridline" }, svg);
    text(svg, m.l - 8, sy(v), minus(fmt.format(v)), "tick-label", "end", "middle");
  }
  for (const v of ticks(0, n, Math.max(3, Math.floor(W / 140)))) {
    if (v < 1 && n > 1) continue;
    text(svg, sx(v), H - m.b + 20, fmt.format(v), "tick-label", sx(v) > W - m.r - 30 ? "end" : "middle");
  }
  el("line", { x1: m.l, x2: W - m.r, y1: sy(yMin), y2: sy(yMin), class: "axis" }, svg);

  // Log: the straight line n·log10 p.
  el("path", { d: `M${sx(0)},${sy(0)}L${sx(n)},${sy(l10)}`, class: "line-1" }, svg);
  // f64: follows until it underflows, then is 0 (off the bottom of any log axis).
  const nEnd = Math.min(n, n0 - 1);
  el("path", { d: `M${sx(0)},${sy(0)}L${sx(nEnd)},${sy(nEnd * Math.log10(p))}`, class: "line-2 dashed" }, svg);

  // The smallest normal f64. Labels go where the line leaves room: below the floor and
  // left of the crossing is always empty, as is below-right when the crossing is far left.
  const floorY = Math.log10(2.2250738585072014e-308);
  if (floorY > yMin) {
    el("line", { x1: m.l, x2: W - m.r, y1: sy(floorY), y2: sy(floorY), class: "floor" }, svg);
    if (n >= n0) {
      const cx = sx(n0), cy = sy(n0 * Math.log10(p)), s = 6;
      el("path", { d: `M${cx - s},${cy - s}L${cx + s},${cy + s}M${cx - s},${cy + s}L${cx + s},${cy - s}`, class: "marker-x" }, svg);
      const lines = [`f64 reaches its floor, 2.2e−308,`, `and returns 0 from n = ${fmt.format(n0)}`];
      // Empty regions: below the floor before the crossing; above the floor after it;
      // and, when the crossing is pinned top-left, the top-right corner above the line.
      let x0, y0, anchor;
      if (cx - m.l > 230) { x0 = cx - 12; y0 = cy + 20; anchor = "end"; }
      else if (cy - 14 - 17 > m.t + 10) { x0 = cx + 12; y0 = cy - 14 - 17; anchor = "start"; }
      else { x0 = W - m.r; y0 = cy + 20; anchor = "end"; }
      lines.forEach((t, i) => text(svg, x0, y0 + i * 17, t, "label", anchor));
    } else {
      text(svg, m.l + 6, sy(floorY) + 18, "smallest normal f64 ≈ 2.2e−308", "label");
    }
  }

  // crosshair
  const cross = el("line", { y1: m.t, y2: H - m.b, class: "cross", visibility: "hidden" }, svg);
  const hit = el("rect", { x: m.l, y: m.t, width: W - m.l - m.r, height: H - m.t - m.b, class: "hit" }, svg);
  hit.addEventListener("pointermove", (e) => {
    const r = svg.getBoundingClientRect();
    const k = Math.max(1, Math.round(sx.invert(((e.clientX - r.left) / r.width) * W)));
    cross.setAttribute("x1", sx(k)); cross.setAttribute("x2", sx(k)); cross.setAttribute("visibility", "visible");
    const fv = Math.pow(p, k);
    showTip(`<b>n = ${fmt.format(k)}</b><br>Log: ${minus(sciFromLog10(k * Math.log10(p)))}<br>f64: ${fv === 0 ? "0" : minus(fv.toExponential(4))}`, e.clientX, e.clientY);
  });
  hit.addEventListener("pointerleave", () => { cross.setAttribute("visibility", "hidden"); hideTip(); });
}

// ---- III. kernels ---------------------------------------------------------------------
function log1mexp(d) { return d <= Math.LN2 ? Math.log(-Math.expm1(-d)) : Math.log1p(-Math.exp(-d)); }

function drawKernels() {
  const svg = document.getElementById("kernel-chart");
  const W = Math.max(300, svg.clientWidth || 560), H = Math.round(Math.max(240, Math.min(340, W * 0.6)));
  svg.setAttribute("viewBox", `0 0 ${W} ${H}`);
  svg.replaceChildren();
  const m = { l: 40, r: 14, t: 14, b: 48 };
  const sx = scale(0, 6, m.l, W - m.r), sy = scale(-4, 1, H - m.b, m.t);
  for (const v of [-4, -3, -2, -1, 0, 1]) {
    el("line", { x1: m.l, x2: W - m.r, y1: sy(v), y2: sy(v), class: v === 0 ? "axis" : "gridline" }, svg);
    text(svg, m.l - 8, sy(v), minus(v), "tick-label", "end", "middle");
  }
  for (let v = 0; v <= 6; v++) text(svg, sx(v), H - m.b + 20, String(v), "tick-label", "middle");
  text(svg, (m.l + W - m.r) / 2, H - 4, "d", "annot", "middle");

  el("line", { x1: sx(Math.LN2), x2: sx(Math.LN2), y1: m.t, y2: H - m.b, class: "floor" }, svg);
  text(svg, sx(Math.LN2) + 6, sy(-3.6), "ln 2: branch switch", "label");

  let a = "", b = "";
  for (let i = 0; i <= 300; i++) {
    const d = (i / 300) * 6;
    a += `${i ? "L" : "M"}${sx(d)},${sy(Math.log1p(Math.exp(-d)))}`;
  }
  let started = false;
  for (let i = 1; i <= 600; i++) {
    const d = (i / 600) * 6, v = log1mexp(d);
    if (v < -4) continue;
    b += `${started ? "L" : "M"}${sx(d)},${sy(v)}`; started = true;
  }
  el("path", { d: a, class: "line-1" }, svg);
  el("path", { d: b, class: "line-2" }, svg);
  text(svg, sx(0.12), sy(Math.LN2) - 8, "ln 2", "label");

  const cross = el("line", { y1: m.t, y2: H - m.b, class: "cross", visibility: "hidden" }, svg);
  const hit = el("rect", { x: m.l, y: m.t, width: W - m.l - m.r, height: H - m.t - m.b, class: "hit" }, svg);
  hit.addEventListener("pointermove", (e) => {
    const r = svg.getBoundingClientRect();
    const d = Math.max(0.001, sx.invert(((e.clientX - r.left) / r.width) * W));
    cross.setAttribute("x1", sx(d)); cross.setAttribute("x2", sx(d)); cross.setAttribute("visibility", "visible");
    showTip(`<b>d = ${d.toFixed(3)}</b><br>ln(1+e⁻ᵈ) = ${Math.log1p(Math.exp(-d)).toFixed(5)}<br>ln(1−e⁻ᵈ) = ${minus(log1mexp(d).toFixed(5))}<br><i>${d <= Math.LN2 ? "branch: ln(−expm1(−d))" : "branch: ln1p(−exp(−d))"}</i>`, e.clientX, e.clientY);
  });
  hit.addEventListener("pointerleave", () => { cross.setAttribute("visibility", "hidden"); hideTip(); });
}

// ---- IV. accuracy ---------------------------------------------------------------------
// From docs/accuracy.md (f64). Units: ε · max(1, |ln x|).
const ACCURACY = [
  ["a * b", 0.491, 0.491, 0.5],
  ["a / b", 0.497, 0.497, 0.5],
  ["powf", 0.477, 0.477, 0.5],
  ["log2", 0.497, 0.497, 0.51],
  ["log10", 0.481, 0.481, 0.51],
  ["from_value", 0.492, 0.492, 1],
  ["to_value", 0.483, 0.52, 1],
  ["a + b", 0.751, 0.663, 1],
  ["log1mexp", 0.524, 0.624, 1.5],
  ["checked_sub", 0.854, 0.854, 2],
  ["Sum", 1.067, 1.067, 2],
  ["sum_two_pass", 1.067, 1.067, 2],
];

function drawAccuracy() {
  const svg = document.getElementById("acc-chart");
  const W = Math.max(320, svg.clientWidth || 900);
  const row = 34, m = { l: W < 520 ? 104 : 124, r: 24, t: 8, b: 52 };
  const H = m.t + m.b + ACCURACY.length * row;
  svg.setAttribute("viewBox", `0 0 ${W} ${H}`);
  svg.replaceChildren();
  const sx = scale(0, 2.1, m.l, W - m.r);
  for (const v of [0, 0.5, 1, 1.5, 2]) {
    el("line", { x1: sx(v), x2: sx(v), y1: m.t, y2: H - m.b, class: v === 0 ? "axis" : "gridline" }, svg);
    text(svg, sx(v), H - m.b + 20, String(v), "tick-label", "middle");
  }
  const bh = 9, gap = 2;
  ACCURACY.forEach(([op, glibc, libm, bound], i) => {
    const y0 = m.t + i * row + row / 2;
    text(svg, m.l - 10, y0, op, "label op", "end", "middle");
    const bars = [[glibc, "bar-1", "platform math (glibc)"], [libm, "bar-2", "libm feature"]];
    bars.forEach(([v, cls, name], j) => {
      const y = y0 - bh - gap / 2 + j * (bh + gap);
      const w = Math.max(1, sx(v) - sx(0));
      // rounded data end, square at the baseline
      el("path", { d: `M${sx(0)},${y}h${w - 3}a3,3 0 0 1 3,3v${bh - 6}a3,3 0 0 1 -3,3h${-(w - 3)}Z`, class: cls }, svg);
      const hit = el("rect", { x: sx(0), y: y - 2, width: Math.max(w, 24), height: bh + 4, class: "hit" }, svg);
      hit.addEventListener("pointermove", (e) => showTip(`<b>${op}</b><br>${name}: ${v.toFixed(3)}<br>bound: ${bound}`, e.clientX, e.clientY));
      hit.addEventListener("pointerleave", hideTip);
    });
    el("line", { x1: sx(bound), x2: sx(bound), y1: y0 - bh - 4, y2: y0 + bh + 4, class: "bound" }, svg);
  });
  text(svg, (m.l + W - m.r) / 2, H - 6, "error, in units of ε·max(1, |ln x|)", "label", "middle");

  const tb = document.querySelector("#acc-table tbody");
  tb.replaceChildren(...ACCURACY.map(([op, g, l, b]) => {
    const tr = document.createElement("tr");
    for (const c of [op, g.toFixed(3), l.toFixed(3), String(b)]) { const td = document.createElement("td"); td.textContent = c; tr.appendChild(td); }
    return tr;
  }));
}

// ---- V. a few lines of Briggs ---------------------------------------------------------
// log10(n) to 14 places, correctly rounded from tests/fixtures/briggs.csv.
const BRIGGS = [
  [10001, "4,00004 34272 7686"],
  [10002, "4,00008 68502 1165"],
  [10003, "4,00013 02688 0523"],
  [10004, "4,00017 36830 5846"],
  [10005, "4,00021 70929 7223"],
];
function drawBriggs() {
  const host = document.getElementById("briggs");
  const t = document.createElement("table");
  t.innerHTML = "<thead><tr><th>Num.</th><th>Logarithmi.</th></tr></thead>";
  const tb = document.createElement("tbody");
  for (const [n, l] of BRIGGS) {
    const tr = document.createElement("tr");
    tr.innerHTML = `<td>${n}</td><td>${l}</td>`;
    tb.appendChild(tr);
  }
  t.appendChild(tb);
  host.replaceChildren(t);
}

// ---- VI. Benford ----------------------------------------------------------------------
function leadingDigits() {
  const pow = new Array(10).fill(0), fac = new Array(10).fill(0);
  const digit = (l10) => Math.min(9, Math.max(1, Math.floor(Math.pow(10, l10 - Math.floor(l10)))));
  for (let k = 1; k <= 10000; k++) pow[digit(k * LOG10_2)]++;
  let s = 0;
  for (let k = 1; k <= 3000; k++) { s += Math.log10(k); fac[digit(s)]++; }
  return { pow: pow.map((c) => c / 10000), fac: fac.map((c) => c / 3000) };
}
const BENFORD = leadingDigits();

function drawBenford() {
  const svg = document.getElementById("benford-chart");
  const W = Math.max(300, svg.clientWidth || 520), H = Math.round(Math.max(240, Math.min(340, W * 0.66)));
  svg.setAttribute("viewBox", `0 0 ${W} ${H}`);
  svg.replaceChildren();
  const m = { l: 40, r: 10, t: 12, b: 32 };
  const band = (W - m.l - m.r) / 9;
  const sy = scale(0, 0.32, H - m.b, m.t);
  for (const v of [0, 0.1, 0.2, 0.3]) {
    el("line", { x1: m.l, x2: W - m.r, y1: sy(v), y2: sy(v), class: v === 0 ? "axis" : "gridline" }, svg);
    text(svg, m.l - 8, sy(v), `${Math.round(v * 100)}%`, "tick-label", "end", "middle");
  }
  const bw = Math.min(16, band * 0.32);
  for (let d = 1; d <= 9; d++) {
    const cx = m.l + band * (d - 0.5);
    const expect = Math.log10(1 + 1 / d);
    [[BENFORD.pow[d], "bar-1", "2ⁿ, n ≤ 10 000", -1], [BENFORD.fac[d], "bar-2", "n!, n ≤ 3 000", 1]].forEach(([v, cls, name, side]) => {
      const x = side < 0 ? cx - bw - 1 : cx + 1;
      const y = sy(v), h = sy(0) - y;
      el("path", { d: `M${x},${sy(0)}V${y + 3}a3,3 0 0 1 3,-3h${bw - 6}a3,3 0 0 1 3,3V${sy(0)}Z`, class: cls }, svg);
      const hit = el("rect", { x: x - 2, y: Math.min(y, sy(0) - 20), width: bw + 4, height: Math.max(h, 20), class: "hit" }, svg);
      hit.addEventListener("pointermove", (e) => showTip(`<b>leading digit ${d}</b><br>${name}: ${(v * 100).toFixed(2)}%<br>Benford: ${(expect * 100).toFixed(2)}%`, e.clientX, e.clientY));
      hit.addEventListener("pointerleave", hideTip);
    });
    el("circle", { cx, cy: sy(expect), r: 4.5, class: "ref-dot" }, svg);
    text(svg, cx, H - m.b + 20, String(d), "tick-label", "middle");
  }
}


// ---- II. the slide rule ---------------------------------------------------------------
const rule = { a: 2, b: 3.5 };
const RULE_TICKS = [
  // [from, to, step, size]: tick spacing within one decade, by sub-range
  [1, 2, 0.05, 0.35], [1, 2, 0.1, 0.55], [2, 5, 0.1, 0.35], [2, 5, 0.5, 0.55], [5, 10, 0.2, 0.35], [5, 10, 0.5, 0.55],
];
function drawRule() {
  const svg = document.getElementById("slide-rule");
  const W = Math.max(320, svg.clientWidth || 900);
  const small = W < 600;
  const H = small ? 150 : 176, pad = small ? 14 : 28, L = W - 2 * pad;
  svg.setAttribute("viewBox", `0 0 ${W} ${H}`);
  svg.replaceChildren();
  const yTop = 30, mid = small ? 84 : 96, yBot = H - 22, tick = small ? 18 : 22;
  const Dx = (v) => pad + (Math.log10(v) / 2) * L;
  const C0 = Dx(rule.a);
  const Cx = (v) => C0 + (Math.log10(v) / 2) * L;

  // stator with the D scale (two decades) along its top edge
  el("rect", { x: pad - 10, y: mid, width: L + 20, height: yBot - mid, rx: 3, class: "rule-body" }, svg);
  const decade = (x, y, dir, decadeIndex) => {
    for (const [lo, hi, step, size] of RULE_TICKS) {
      for (let v = lo; v < hi - 1e-9; v += step) {
        if (Math.abs(v - Math.round(v)) < 1e-9) continue;
        const xv = x(v * Math.pow(10, decadeIndex));
        el("line", { x1: xv, x2: xv, y1: y, y2: y + dir * tick * size, class: "rule-tick minor" }, svg);
      }
    }
    for (let k = 1; k <= 10; k++) {
      const xv = x(k * Math.pow(10, decadeIndex));
      el("line", { x1: xv, x2: xv, y1: y, y2: y + dir * tick, class: "rule-tick" }, svg);
    }
  };
  decade(Dx, mid, 1, 0);
  decade(Dx, mid, 1, 1);
  for (const k of [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100]) {
    if (small && k > 10 && k !== 20 && k !== 50 && k !== 100) continue;
    if (small && k < 10 && ![1, 2, 3, 5].includes(k)) continue;
    text(svg, Dx(k), mid + tick + 14, String(k), "rule-num", "middle");
  }
  text(svg, pad - 6, yBot - 6, "D", "rule-name", "end");

  // the slide with the C scale (one decade) along its bottom edge
  el("rect", { x: C0 - 8, y: yTop, width: L / 2 + 16, height: mid - yTop, rx: 3, class: "rule-slide" }, svg);
  decade(Cx, mid, -1, 0);
  for (let k = 1; k <= 10; k++) {
    if (small && ![1, 2, 3, 5, 10].includes(k)) continue;
    text(svg, Cx(k), mid - tick - 6, String(k), k === 1 || k === 10 ? "rule-num red" : "rule-num", "middle");
  }
  text(svg, C0 - 12, yTop + 16, "C", "rule-name", "end");

  // the cursor: glass and hairline at b on C, reading a·b on D
  const xb = Cx(rule.b);
  el("rect", { x: xb - 16, y: yTop - 12, width: 32, height: yBot - yTop + 18, rx: 3, class: "rule-cursor" }, svg);
  el("line", { x1: xb, x2: xb, y1: yTop - 12, y2: yBot + 6, class: "rule-hair" }, svg);
  el("path", { d: `M${C0 - 5},${mid - 1}L${C0 + 5},${mid - 1}L${C0},${mid + 7}Z`, class: "rule-mark" }, svg);

  const ab = rule.a * rule.b;
  tex(document.getElementById("rule-readout"),
    `\\log a + \\log b = \\log ab:\\quad ${rule.a.toFixed(2)} \\times ${rule.b.toFixed(2)} = ${ab.toFixed(ab < 10 ? 3 : 2)}`);

  // geometry for the event handlers in initRule
  rule.geom = { W, L, C0, slideTop: yTop, slideBottom: mid };
}
function initRule() {
  const svg = document.getElementById("slide-rule");
  svg.setAttribute("tabindex", "0");
  const pos = (e) => {
    const r = svg.getBoundingClientRect();
    return { x: ((e.clientX - r.left) / r.width) * rule.geom.W, y: ((e.clientY - r.top) / r.height) * r.height * (rule.geom.W / r.width) };
  };
  const setB = (x) => { const g = rule.geom; rule.b = Math.min(10, Math.max(1, Math.pow(10, ((x - g.C0) / g.L) * 2))); drawRule(); };
  let drag = null;
  svg.addEventListener("pointerdown", (e) => {
    const p = pos(e), g = rule.geom;
    const onSlide = p.x >= g.C0 - 8 && p.x <= g.C0 + g.L / 2 + 8 && p.y >= g.slideTop && p.y <= g.slideBottom;
    svg.setPointerCapture(e.pointerId);
    if (onSlide) drag = { x: p.x, a: rule.a };
    else setB(p.x);
  });
  svg.addEventListener("pointermove", (e) => {
    const p = pos(e);
    if (drag) {
      rule.a = Math.min(10, Math.max(1, drag.a * Math.pow(10, ((p.x - drag.x) / rule.geom.L) * 2)));
      drawRule();
    } else if (e.buttons === 0 || e.pointerType !== "mouse") {
      setB(p.x);
    }
  });
  const stop = () => { drag = null; };
  svg.addEventListener("pointerup", stop);
  svg.addEventListener("pointercancel", stop);
  svg.addEventListener("keydown", (e) => {
    const f = Math.pow(10, e.shiftKey ? 0.02 : 0.005);
    const key = { ArrowRight: ["a", f], ArrowLeft: ["a", 1 / f], ArrowUp: ["b", f], ArrowDown: ["b", 1 / f] }[e.key];
    if (!key) return;
    e.preventDefault();
    rule[key[0]] = Math.min(10, Math.max(1, rule[key[0]] * key[1]));
    drawRule();
  });
}

// ---- IV. the calculator (the crate, compiled to WebAssembly) ---------------------------
let calc = null;
async function initCalc() {
  const input = document.getElementById("calc-input");
  const value = document.getElementById("calc-value");
  try {
    const resp = await fetch("calc.wasm");
    if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
    const { instance } = await WebAssembly.instantiate(await resp.arrayBuffer(), {});
    calc = instance.exports;
  } catch (err) {
    value.className = "calc-value error";
    value.textContent = "The calculator needs WebAssembly, which this browser did not load.";
    return;
  }
  let timer = 0;
  const run = () => renderCalc(input.value);
  input.addEventListener("input", () => { clearTimeout(timer); timer = setTimeout(run, 120); });
  for (const b of document.querySelectorAll(".chips button")) {
    b.addEventListener("click", () => { input.value = b.textContent; run(); input.focus(); });
  }
  run();
}

function evaluateWasm(src) {
  const bytes = new TextEncoder().encode(src);
  const cap = Math.max(1, bytes.length);
  const p = calc.alloc(cap);
  new Uint8Array(calc.memory.buffer, p, bytes.length).set(bytes);
  const packed = calc.evaluate(p, bytes.length);
  const ptr = Number(packed >> 32n), len = Number(packed & 0xffffffffn);
  const json = new TextDecoder().decode(new Uint8Array(calc.memory.buffer, ptr, len));
  calc.dealloc(ptr, len);
  calc.dealloc(p, cap);
  return JSON.parse(json);
}

function renderCalc(src) {
  const value = document.getElementById("calc-value");
  const rows = document.getElementById("calc-rows");
  let r;
  try { r = evaluateWasm(src); } catch (e) { r = { ok: false, error: String(e) }; }
  rows.replaceChildren();
  if (!r.ok) {
    value.className = "calc-value error";
    value.textContent = r.error;
    return;
  }
  value.className = "calc-value";
  value.textContent = r.value;
  const add = (k, v) => {
    const dt = document.createElement("dt"); dt.textContent = k;
    const dd = document.createElement("dd"); dd.textContent = v;
    rows.append(dt, dd);
  };
  add("relative error ≤", `${r.err}  (${r.digits} significant digit${r.digits === "1" ? "" : "s"})`);
  add("stored value, in full", r.exact);
  add("stored ln |x|", r.ln);
  add("as a power of 2", r.pow2);
  add("as a power of 10", r.pow10);
  add("to_value() as f64", r.f64);
}

// ---- navigation and copy buttons -------------------------------------------------------
function initNav() {
  const links = [...document.querySelectorAll('nav a[href^="#"]')]
    .map((a) => [document.getElementById(a.getAttribute("href").slice(1)), a])
    .filter(([s]) => s);
  const update = () => {
    // The current section is the last one whose top has passed a line 30% down the window.
    const line = innerHeight * 0.3;
    let current = null;
    for (const [s, a] of links) if (s.getBoundingClientRect().top <= line) current = a;
    for (const [, a] of links) a.classList.toggle("active", a === current);
  };
  addEventListener("scroll", update, { passive: true });
  update();
}

function initCopy() {
  for (const pre of document.querySelectorAll("pre.code")) {
    const b = document.createElement("button");
    b.type = "button"; b.className = "copy"; b.textContent = "Copy";
    b.addEventListener("click", async () => {
      try { await navigator.clipboard.writeText(pre.querySelector("code").innerText); b.textContent = "Copied"; }
      catch (_) { b.textContent = "Select and copy"; }
      setTimeout(() => (b.textContent = "Copy"), 1500);
    });
    pre.appendChild(b);
  }
}

// ---- wiring ---------------------------------------------------------------------------
function drawAll() {
  drawHyperbola();
  drawRule();
  drawUnderflow();
  drawKernels();
  drawAccuracy();
  drawBenford();
}

function init() {
  initTheme();
  renderMath();
  drawHero();
  drawBriggs();
  initHyperbola();
  initRule();
  initNav();
  initCopy();
  initCalc();
  document.getElementById("hyp-x").addEventListener("input", (e) => { hyp.x = +e.target.value; drawHyperbola(); });
  for (const id of ["uf-p", "uf-n"]) document.getElementById(id).addEventListener("input", () => { readUnderflow(); drawUnderflow(); });
  readUnderflow();
  drawAll();
  let pending = 0;
  new ResizeObserver(() => { cancelAnimationFrame(pending); pending = requestAnimationFrame(drawAll); }).observe(document.body);
}

if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", init);
else init();
