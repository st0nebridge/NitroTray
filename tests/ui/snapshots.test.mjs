import { test } from 'node:test';
import { loadPage } from './helpers/dom.mjs';
import { lines, matchSnapshot, recordingContext } from './helpers/snapshot.mjs';
import { ICONS } from '../../src/ui/web/js/icons.js';
import * as settingsView from '../../src/ui/web/js/settings.js';
import { createMonitor } from '../../src/ui/web/js/monitor.js';
import { createCurveEditor } from '../../src/ui/web/js/curve.js';
import { renderPreview } from '../../src/ui/web/js/graph-preview.js';

// Goldens lock the visual contract that was measured against the reference image;
// regenerate with NITROTRAY_WRITE_FIXTURES=1 only after reviewing the rendered result.

test('icon family markup', () => {
  const text = Object.entries(ICONS).map(([name, make]) => `${name}\n${lines(make())}`).join('\n\n');
  matchSnapshot('icons', `${text}\n\nfan(2)\n${lines(ICONS.fan(2))}\n`);
});

test('settings form markup', () => {
  const win = loadPage('window.html');
  const form = win.document.getElementById('settings-form');
  settingsView.buildForm(form);
  matchSnapshot('settings-form', `${lines(form.innerHTML)}\n`);
});

function monitorLog({ dpr, width, height, unit }) {
  const win = loadPage('window.html');
  const { ctx, log } = recordingContext();
  win.HTMLCanvasElement.prototype.getContext = (type) => { log.push(`getContext(${type})`); return ctx; };
  if (width) {
    Object.defineProperty(win.HTMLElement.prototype, 'clientWidth', { configurable: true, get: () => width });
    Object.defineProperty(win.HTMLElement.prototype, 'clientHeight', { configurable: true, get: () => height });
  }
  globalThis.devicePixelRatio = dpr;
  const box = win.document.getElementById('charts');
  const m = createMonitor(box, win.document);
  m.setUnit(unit);
  m.setHistory({
    cpu_temp: [70, 75, null, 85], gpu_temp: [60, 62, 64, 82], cpu_util: [10, 21], gpu_util: [90, 93],
    cpu_fan: [6000, 7317], gpu_fan: [7000, 7692], cpu_clock: [3900, 3947], gpu_clock: [2500, 2565], gpu_power: [80.25, 95.46],
  });
  m.render();
  delete globalThis.devicePixelRatio;
  const sizes = [...box.querySelectorAll('canvas')].map((c) => `${c.width}x${c.height}`).join(' ');
  return `${lines(box.innerHTML)}\n\ncanvas ${sizes}\n${log.join('\n')}\n`;
}

test('monitoring charts: markup and drawing at default size', () => {
  matchSnapshot('monitor-default', monitorLog({ dpr: undefined }));
});

test('monitoring charts: drawing at 2x DPI with a measured canvas, in Fahrenheit', () => {
  matchSnapshot('monitor-2x', monitorLog({ dpr: 2, width: 400, height: 200, unit: 'fahrenheit' }));
});

test('curve editor plot with live marker, clamped markers and table', () => {
  const win = loadPage('window.html');
  const card = win.document.querySelector('.curve-card[data-fan="cpu"]');
  const editor = createCurveEditor(card);
  const out = [];
  for (const live of [85, 10, 120, null]) {
    editor.setLive(live);
    out.push(`live ${live}\n${lines(card.querySelector('svg').innerHTML)}`);
  }
  out.push(`table\n${lines(card.querySelector('.points').innerHTML)}`);
  matchSnapshot('curve-editor', `${out.join('\n\n')}\n`);
});

test('tray graph preview drawing', () => {
  const s = {
    background_color: '#000000', show_temperature: true, show_fan: true, fill: true,
    temperature_color: '#FF2A2A', fan_color: '#22D3EE', temperature_min_c: 30, temperature_max_c: 100, fan_max_rpm: 0,
  };
  const out = [];
  for (const size of [16, 32]) {
    const { ctx, log } = recordingContext();
    const canvas = { getContext: (type) => { log.push(`getContext(${type})`); return ctx; } };
    const temps = Array.from({ length: size + 4 }, (_, i) => 50 + i);
    temps[size] = null;
    renderPreview(canvas, s, temps, [5000, null, Number.NaN, 7317], size);
    out.push(`size ${size} canvas ${canvas.width}x${canvas.height}\n${log.join('\n')}`);
  }
  const { ctx, log } = recordingContext();
  renderPreview({ getContext: () => ctx }, { ...s, fan_max_rpm: 8000, fill: false }, [70], [4000, 8000], 16);
  out.push(`fixed scale\n${log.join('\n')}`);
  matchSnapshot('graph-preview', `${out.join('\n\n')}\n`);
});
