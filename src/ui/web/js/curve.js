/**
 * @module curve
 * @description Custom fan-curve editor: constrained dragging, validation that mirrors
 *              the Rust rules (controls::fan_curve), and an SVG plot with a live-temperature marker.
 *
 * @input  Curve points `[{temp_c, duty_pct}]`, pointer/keyboard edits, current temperature.
 * @output Valid point lists via `getPoints()`; human-readable validation messages.
 * @dependencies none
 */

export const RULES = { MIN_POINTS: 2, MAX_POINTS: 8, MIN_DUTY: 20, MAX_DUTY: 100, MIN_TEMP: 20, MAX_TEMP: 100, SAFETY_C: 90 };

export const DEFAULT_POINTS = [[40, 25], [50, 35], [60, 50], [70, 70], [80, 90], [90, 100]].map(([temp_c, duty_pct]) => ({ temp_c, duty_pct }));

/** Same wording as Rust `CurveError` Display. */
export function validate(points) {
  const R = RULES;
  if (points.length < R.MIN_POINTS) return `a curve needs at least ${R.MIN_POINTS} points`;
  if (points.length > R.MAX_POINTS) return `a curve allows at most ${R.MAX_POINTS} points`;
  for (let i = 0; i < points.length; i += 1) {
    const p = points[i];
    if (!Number.isInteger(p.temp_c) || p.temp_c < R.MIN_TEMP || p.temp_c > R.MAX_TEMP) return `point ${i + 1} temperature must be ${R.MIN_TEMP}-${R.MAX_TEMP} °C`;
    if (!Number.isInteger(p.duty_pct) || p.duty_pct < R.MIN_DUTY || p.duty_pct > R.MAX_DUTY) return `point ${i + 1} duty must be ${R.MIN_DUTY}-${R.MAX_DUTY} %`;
    if (i > 0 && p.temp_c <= points[i - 1].temp_c) return `point ${i + 1} must be hotter than the point before it`;
    if (i > 0 && p.duty_pct < points[i - 1].duty_pct) return `point ${i + 1} must not lower the fan speed`;
  }
  return null;
}

/** Mirrors `FanCurve::duty_at`. */
export function dutyAt(points, t) {
  if (!Number.isFinite(t) || t >= RULES.SAFETY_C || points.length === 0) return RULES.MAX_DUTY;
  const first = points[0];
  const last = points[points.length - 1];
  if (t <= first.temp_c) return Math.max(first.duty_pct, RULES.MIN_DUTY);
  if (t >= last.temp_c) return Math.max(last.duty_pct, RULES.MIN_DUTY);
  for (let i = 1; i < points.length; i += 1) {
    const a = points[i - 1];
    const b = points[i];
    if (t <= b.temp_c) {
      const d = a.duty_pct + ((t - a.temp_c) / (b.temp_c - a.temp_c)) * (b.duty_pct - a.duty_pct);
      return Math.min(RULES.MAX_DUTY, Math.max(RULES.MIN_DUTY, Math.round(d)));
    }
  }
  return RULES.MAX_DUTY;
}

/** Moves point `i`, clamped between its neighbours so the curve stays valid. */
export function clampMove(points, i, temp, duty) {
  const R = RULES;
  const prev = points[i - 1];
  const next = points[i + 1];
  const tLo = prev ? prev.temp_c + 1 : R.MIN_TEMP;
  const tHi = next ? next.temp_c - 1 : R.MAX_TEMP;
  const dLo = prev ? prev.duty_pct : R.MIN_DUTY;
  const dHi = next ? next.duty_pct : R.MAX_DUTY;
  const out = points.map((p) => ({ ...p }));
  out[i] = {
    temp_c: Math.round(Math.min(tHi, Math.max(tLo, temp))),
    duty_pct: Math.round(Math.min(dHi, Math.max(dLo, Math.max(R.MIN_DUTY, Math.min(R.MAX_DUTY, duty))))),
  };
  return out;
}

/** Inserts a point in the middle of the widest temperature gap. */
export function addPoint(points) {
  if (points.length >= RULES.MAX_POINTS) return points;
  let widest = 1;
  for (let i = 1; i < points.length; i += 1) {
    if (points[i].temp_c - points[i - 1].temp_c > points[widest].temp_c - points[widest - 1].temp_c) widest = i;
  }
  const a = points[widest - 1];
  const b = points[widest];
  if (b.temp_c - a.temp_c < 2) return points;
  const mid = { temp_c: Math.round((a.temp_c + b.temp_c) / 2), duty_pct: Math.round((a.duty_pct + b.duty_pct) / 2) };
  return [...points.slice(0, widest), mid, ...points.slice(widest)];
}

export function removePoint(points, i) {
  if (points.length <= RULES.MIN_POINTS) return points;
  return points.filter((_, k) => k !== i);
}

// Plot geometry in SVG units (viewBox 0 0 420 260).
export const PLOT = { x0: 40, x1: 410, y0: 232, y1: 14 };
export const xOf = (t) => PLOT.x0 + ((t - RULES.MIN_TEMP) / (RULES.MAX_TEMP - RULES.MIN_TEMP)) * (PLOT.x1 - PLOT.x0);
export const yOf = (d) => PLOT.y0 - (d / 100) * (PLOT.y0 - PLOT.y1);
export const tOf = (x) => RULES.MIN_TEMP + ((x - PLOT.x0) / (PLOT.x1 - PLOT.x0)) * (RULES.MAX_TEMP - RULES.MIN_TEMP);
export const dOf = (y) => ((PLOT.y0 - y) / (PLOT.y0 - PLOT.y1)) * 100;

export function createCurveEditor(card, { onChange } = {}) {
  const svg = card.querySelector('svg');
  const table = card.querySelector('.points');
  let points = DEFAULT_POINTS.map((p) => ({ ...p }));
  let live = null;
  let drag = -1;

  function draw() {
    const grid = [];
    for (let t = 20; t <= 100; t += 10) grid.push(`<line class="grid" x1="${xOf(t)}" x2="${xOf(t)}" y1="${PLOT.y1}" y2="${PLOT.y0}"/><text class="axis" x="${xOf(t)}" y="${PLOT.y0 + 16}" text-anchor="middle">${t}°</text>`);
    for (let d = 0; d <= 100; d += 25) grid.push(`<line class="grid" x1="${PLOT.x0}" x2="${PLOT.x1}" y1="${yOf(d)}" y2="${yOf(d)}"/><text class="axis" x="${PLOT.x0 - 6}" y="${yOf(d) + 3}" text-anchor="end">${d}%</text>`);
    const path = points.map((p, i) => `${i ? 'L' : 'M'}${xOf(p.temp_c).toFixed(1)} ${yOf(p.duty_pct).toFixed(1)}`).join(' ');
    const area = `${path} L${xOf(points.at(-1).temp_c)} ${PLOT.y0} L${xOf(points[0].temp_c)} ${PLOT.y0}Z`;
    const liveMark = Number.isFinite(live)
      ? `<line class="live" x1="${xOf(Math.min(100, Math.max(20, live)))}" x2="${xOf(Math.min(100, Math.max(20, live)))}" y1="${PLOT.y1}" y2="${PLOT.y0}"/><circle class="live-dot" r="4" cx="${xOf(Math.min(100, Math.max(20, live)))}" cy="${yOf(dutyAt(points, live))}"/>`
      : '';
    svg.innerHTML = `<rect class="safety" x="${xOf(RULES.SAFETY_C)}" y="${PLOT.y1}" width="${PLOT.x1 - xOf(RULES.SAFETY_C)}" height="${PLOT.y0 - PLOT.y1}"/>${grid.join('')}<path class="area" d="${area}"/><path class="path" d="${path}"/>${liveMark}${points
      .map((p, i) => `<circle class="pt" tabindex="0" data-i="${i}" r="6.5" cx="${xOf(p.temp_c)}" cy="${yOf(p.duty_pct)}"><title>${p.temp_c} °C → ${p.duty_pct} %</title></circle>`)
      .join('')}`;
    table.innerHTML = points
      .map((p, i) => `<label>#${i + 1}<span></span><input type="number" data-i="${i}" data-f="temp_c" value="${p.temp_c}" aria-label="Point ${i + 1} temperature"><input type="number" data-i="${i}" data-f="duty_pct" value="${p.duty_pct}" aria-label="Point ${i + 1} duty"></label>`)
      .join('');
  }

  function set(next) {
    points = next;
    draw();
    onChange?.(points);
  }

  const toSvg = (e) => {
    const r = svg.getBoundingClientRect();
    return [((e.clientX - r.left) / r.width) * 420, ((e.clientY - r.top) / r.height) * 260];
  };
  svg.addEventListener('pointerdown', (e) => {
    const pt = e.target.closest('.pt');
    if (!pt) return;
    drag = Number(pt.dataset.i);
    svg.setPointerCapture?.(e.pointerId);
  });
  svg.addEventListener('pointermove', (e) => {
    if (drag < 0) return;
    const [x, y] = toSvg(e);
    set(clampMove(points, drag, tOf(x), dOf(y)));
  });
  svg.addEventListener('pointerup', () => { drag = -1; });
  svg.addEventListener('keydown', (e) => {
    const pt = e.target.closest('.pt');
    if (!pt) return;
    const i = Number(pt.dataset.i);
    const p = points[i];
    const moves = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, 1], ArrowDown: [0, -1] };
    if (moves[e.key]) {
      e.preventDefault();
      set(clampMove(points, i, p.temp_c + moves[e.key][0], p.duty_pct + moves[e.key][1]));
      svg.querySelector(`.pt[data-i="${i}"]`)?.focus();
    } else if (e.key === 'Delete') {
      set(removePoint(points, i));
    } else if (e.key === 'Insert' || e.key === '+') {
      set(addPoint(points));
    }
  });
  table.addEventListener('change', (e) => {
    const input = e.target.closest('input');
    if (!input) return;
    const i = Number(input.dataset.i);
    const next = points.map((p) => ({ ...p }));
    next[i][input.dataset.f] = Math.round(Number(input.value));
    set(next);
  });

  draw();
  return {
    setPoints(p) { points = p.map((x) => ({ ...x })); draw(); },
    getPoints() { return points.map((p) => ({ ...p })); },
    setLive(t) { live = t; draw(); },
    reset() { set(DEFAULT_POINTS.map((p) => ({ ...p }))); },
  };
}
