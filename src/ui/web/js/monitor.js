/**
 * @module monitor
 * @description Full monitoring view: temperature, utilisation, fan, clock and power history.
 *
 * @input  A HistoryView from Rust (oldest-first arrays) and per-tick samples; unit preference.
 * @output Canvas line charts with legends; gaps drawn as breaks, never interpolated.
 * @dependencies none
 */

export const CAPACITY = 300;

export const CHARTS = [
  { id: 'temps', title: 'Temperature', unit: 'temp', min: 30, max: 100,
    lines: [{ key: 'cpu_temp', label: 'CPU', color: '#ff252d' }, { key: 'gpu_temp', label: 'GPU', color: '#ffa53a' }] },
  { id: 'util', title: 'Utilisation', unit: '%', min: 0, max: 100,
    lines: [{ key: 'cpu_util', label: 'CPU', color: '#ff252d' }, { key: 'gpu_util', label: 'GPU', color: '#ffa53a' }] },
  { id: 'fans', title: 'Fan speed', unit: 'RPM', min: 0, max: 'auto',
    lines: [{ key: 'cpu_fan', label: 'CPU', color: '#22d3ee' }, { key: 'gpu_fan', label: 'GPU', color: '#3ddc84' }] },
  { id: 'clocks', title: 'Clock', unit: 'MHz', min: 0, max: 'auto',
    lines: [{ key: 'cpu_clock', label: 'CPU', color: '#ff252d' }, { key: 'gpu_clock', label: 'GPU', color: '#ffa53a' }] },
  { id: 'power', title: 'GPU board power', unit: 'W', min: 0, max: 'auto',
    lines: [{ key: 'gpu_power', label: 'GPU', color: '#ffa53a' }] },
];

/** Rounds up to 1/2/2.5/5 × 10^n. */
export function niceMax(v) {
  if (!Number.isFinite(v) || v <= 0) return 1;
  const exp = 10 ** Math.floor(Math.log10(v));
  const f = v / exp;
  const step = [1, 2, 2.5, 5, 10].find((s) => f <= s);
  return step * exp;
}

export const toF = (c) => (c * 9) / 5 + 32;

/** Y range for a chart given its values (already unit-converted). */
export function yRange(chart, seriesValues, fahrenheit = false) {
  let min = chart.min;
  let max = chart.max;
  if (chart.unit === 'temp' && fahrenheit) { min = toF(min); max = toF(max); }
  if (max === 'auto') {
    const peak = Math.max(0, ...seriesValues.flat().filter(Number.isFinite));
    max = niceMax(peak * 1.1);
  }
  return [min, max];
}

/** Maps samples to canvas points; `null` marks a gap. Newest sample at the right edge. */
export function toPoints(values, width, height, [min, max], capacity = CAPACITY) {
  const span = max - min || 1;
  const step = width / Math.max(1, capacity - 1);
  const offset = capacity - values.length;
  return values.map((v, i) => {
    if (!Number.isFinite(v)) return null;
    const t = Math.min(1, Math.max(0, (v - min) / span));
    return [(offset + i) * step, height - t * height];
  });
}

export function formatValue(v, unit) {
  if (!Number.isFinite(v)) return '--';
  if (unit === 'W') return v.toFixed(1);
  return String(Math.round(v));
}

export function createMonitor(container, doc = globalThis.document) {
  const history = {};
  let fahrenheit = false;
  const cards = CHARTS.map((chart) => {
    const card = doc.createElement('div');
    card.className = 'chart-card';
    card.innerHTML = `<div class="chart-head"><h2>${chart.title}</h2><div class="legend">${chart.lines
      .map((l) => `<span><i style="background:${l.color}"></i>${l.label}<b data-key="${l.key}">--</b></span>`)
      .join('')}</div></div><canvas></canvas>`;
    container.appendChild(card);
    return { chart, card, canvas: card.querySelector('canvas') };
  });

  const series = (key) => {
    const raw = history[key] || [];
    const chart = CHARTS.find((c) => c.lines.some((l) => l.key === key));
    return chart?.unit === 'temp' && fahrenheit ? raw.map((v) => (Number.isFinite(v) ? toF(v) : v)) : raw;
  };

  function draw({ chart, card, canvas }) {
    const values = chart.lines.map((l) => series(l.key));
    const unitLabel = chart.unit === 'temp' ? (fahrenheit ? '°F' : '°C') : chart.unit;
    for (const l of chart.lines) {
      const v = series(l.key).at(-1);
      card.querySelector(`[data-key="${l.key}"]`).textContent = `${formatValue(v, chart.unit)} ${unitLabel}`;
    }
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    const dpr = globalThis.devicePixelRatio || 1;
    const w = canvas.clientWidth || 360;
    const h = canvas.clientHeight || 170;
    canvas.width = Math.round(w * dpr);
    canvas.height = Math.round(h * dpr);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, w, h);
    const range = yRange(chart, values, fahrenheit);
    const plotW = w - 44;
    ctx.font = '10px Segoe UI, system-ui';
    ctx.fillStyle = 'rgba(160,166,172,.9)';
    ctx.strokeStyle = 'rgba(255,255,255,.08)';
    for (let g = 0; g <= 4; g += 1) {
      const y = 4 + ((h - 8) * g) / 4;
      ctx.beginPath(); ctx.moveTo(0, y); ctx.lineTo(plotW, y); ctx.stroke();
      const label = range[1] - ((range[1] - range[0]) * g) / 4;
      ctx.fillText(formatValue(label, chart.unit), plotW + 6, y + 3);
    }
    chart.lines.forEach((l, i) => {
      const pts = toPoints(values[i], plotW, h - 8, range).map((p) => (p ? [p[0], p[1] + 4] : null));
      ctx.strokeStyle = l.color;
      ctx.lineWidth = 1.8;
      ctx.beginPath();
      let pen = false;
      for (const p of pts) {
        if (!p) { pen = false; continue; }
        if (pen) ctx.lineTo(p[0], p[1]); else ctx.moveTo(p[0], p[1]);
        pen = true;
      }
      ctx.stroke();
    });
  }

  return {
    setUnit(unit) { fahrenheit = unit === 'fahrenheit'; },
    setHistory(view) {
      for (const k of Object.keys(view || {})) if (Array.isArray(view[k])) history[k] = view[k].slice(-CAPACITY);
    },
    push(sample) {
      for (const [k, v] of Object.entries(sample || {})) {
        if (k === 'timestamp') continue;
        const arr = (history[k] ||= []);
        arr.push(Number.isFinite(v) ? v : null);
        if (arr.length > CAPACITY) arr.shift();
      }
    },
    render() { cards.forEach(draw); },
    history,
  };
}
