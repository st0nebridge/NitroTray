/**
 * @module flyout-view
 * @description Hover flyout view: the CPU or GPU icon's temperature/fan history drawn large, live values
 *              coloured by temperature band, and min/avg/max over the graph window.
 *
 * @input  `FlyoutState` from Rust (`src/ui/flyout.rs`): series oldest-first, ranges, colours, stats.
 * @output DOM text and a canvas drawing; gaps are breaks, never interpolated.
 * @dependencies format, icons, monitor (toPoints)
 */
import { DASH, int, levelClass } from './format.js';
import { ICONS } from './icons.js';
import { toPoints } from './monitor.js';

/** Opacity of the temperature area fill (the tray icon's FILL_ALPHA 110/255). */
export const FILL_ALPHA = 0.43;

/** `#RRGGBB` → `rgba(r,g,b,a)`; anything else falls back to the given colour unchanged. */
export function withAlpha(hex, alpha) {
  const m = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(hex || '');
  if (!m) return hex;
  const [r, g, b] = m.slice(1).map((h) => parseInt(h, 16));
  return `rgba(${r},${g},${b},${alpha})`;
}

/** "last 45 s" / "last 4 min" / "last 1 h 20 min"; empty before there is any span. */
export function spanLabel(seconds) {
  if (!Number.isFinite(seconds) || seconds <= 0) return '';
  if (seconds < 90) return `last ${Math.round(seconds)} s`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `last ${minutes} min`;
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  return m ? `last ${h} h ${m} min` : `last ${h} h`;
}

export function statsText(stats, symbol) {
  const t = (v) => (Number.isFinite(v) ? `${Math.round(v)}${symbol}` : DASH);
  return `Min ${t(stats?.min)} · Avg ${t(stats?.avg)} · Max ${t(stats?.max)}`;
}

/** Splits mapped points into unbroken runs (a `null` point ends a run). */
export function runs(points) {
  const out = [];
  let cur = [];
  for (const p of points) {
    if (p) cur.push(p);
    else if (cur.length) { out.push(cur); cur = []; }
  }
  if (cur.length) out.push(cur);
  return out;
}

/** Draws grid, temperature area + line and the fan line into a `w`×`h` CSS-pixel context. */
export function drawGraph(ctx, w, h, state) {
  ctx.clearRect(0, 0, w, h);
  ctx.strokeStyle = 'rgba(128,136,144,.18)';
  ctx.lineWidth = 1;
  for (let g = 1; g < 4; g += 1) {
    const y = Math.round((h * g) / 4) + 0.5;
    ctx.beginPath(); ctx.moveTo(0, y); ctx.lineTo(w, y); ctx.stroke();
  }
  const cap = state.capacity || 300;
  const temps = runs(toPoints(state.temps || [], w, h, state.temp_range, cap));
  for (const run of temps) {
    if (state.fill) {
      ctx.fillStyle = withAlpha(state.temp_color, FILL_ALPHA);
      ctx.beginPath();
      ctx.moveTo(run[0][0], h);
      for (const [x, y] of run) ctx.lineTo(x, y);
      ctx.lineTo(run.at(-1)[0], h);
      ctx.closePath();
      ctx.fill();
    }
    stroke(ctx, run, state.temp_color, 1.6);
  }
  if (state.show_fan) {
    for (const run of runs(toPoints(state.fans || [], w, h, [0, state.fan_max], cap))) stroke(ctx, run, state.fan_color, 1.4);
  }
}

function stroke(ctx, run, color, width) {
  ctx.strokeStyle = color;
  ctx.lineWidth = width;
  ctx.lineJoin = 'round';
  ctx.beginPath();
  run.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
  if (run.length === 1) ctx.lineTo(run[0][0] + 1, run[0][1]);
  ctx.stroke();
}

export function createFlyoutView(root, win = globalThis) {
  const $ = (id) => root.querySelector(`#${id}`);
  let kind = null;
  return {
    render(state) {
      if (!state) return;
      const symbol = state.prefs?.unit_symbol || '°C';
      if (state.kind !== kind && ICONS[state.kind]) {
        kind = state.kind;
        $('fly-icon').innerHTML = ICONS[state.kind]();
      }
      $('fly-title').textContent = state.title;
      const span = spanLabel(state.span_s);
      $('fly-span').textContent = span ? `Temperature and fan speed · ${span}` : 'Temperature and fan speed';
      const temp = $('fly-temp');
      temp.className = `fly-temp num ${levelClass(state.level)}`;
      temp.querySelector('b').textContent = int(state.temp);
      temp.querySelector('small').textContent = symbol;
      const fan = $('fly-fan');
      fan.hidden = !state.show_fan;
      fan.querySelector('b').textContent = int(state.fan_rpm);
      $('fly-stats').textContent = statsText(state.stats, '°');
      $('axis-top').textContent = `${int(state.temp_range[1])}${symbol}`;
      $('axis-bottom').textContent = `${int(state.temp_range[0])}${symbol}`;
      $('axis-fan').textContent = state.show_fan ? `${int(state.fan_max)} RPM` : '';
      $('sw-temp').style.background = state.temp_color;
      $('sw-fan').style.background = state.fan_color;
      $('sw-fan').hidden = !state.show_fan;
      $('legend-fan').hidden = !state.show_fan;
      const canvas = $('fly-canvas');
      const ctx = canvas.getContext('2d');
      if (!ctx) return;
      const dpr = win.devicePixelRatio || 1;
      const w = canvas.clientWidth || 312;
      const h = canvas.clientHeight || 122;
      canvas.width = Math.round(w * dpr);
      canvas.height = Math.round(h * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      drawGraph(ctx, w, h, state);
    },
  };
}
