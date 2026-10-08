import { test } from 'node:test';
import assert from 'node:assert/strict';
import { loadPage } from './helpers/dom.mjs';
import { lines, matchSnapshot, recordingContext } from './helpers/snapshot.mjs';
import { createFlyoutView, drawGraph, FILL_ALPHA, runs, spanLabel, statsText, withAlpha } from '../../src/ui/web/js/flyout-view.js';

const cpuState = () => ({
  kind: 'cpu', title: 'CPU', temp: 92, level: 'hot', fan_rpm: 4400,
  temps: [60, 66, null, 88, 92], fans: [3000, 3200, null, 4100, 4400],
  capacity: 300, temp_range: [30, 100], fan_max: 4840,
  temp_color: '#FF2A2A', fan_color: '#22D3EE', fill: true, show_fan: true,
  stats: { min: 60, avg: 76.5, max: 92 }, span_s: 600,
  prefs: { theme: 'dark', accent: 'normal', reduced_motion: false, transparency: false, unit_symbol: '°C', unit: 'celsius' },
});

function render(state, { size } = {}) {
  const win = loadPage('flyout.html');
  const { ctx, log } = recordingContext();
  win.HTMLCanvasElement.prototype.getContext = () => ctx;
  if (size) {
    Object.defineProperty(win.HTMLElement.prototype, 'clientWidth', { configurable: true, get: () => size[0] });
    Object.defineProperty(win.HTMLElement.prototype, 'clientHeight', { configurable: true, get: () => size[1] });
  }
  const view = createFlyoutView(win.document.getElementById('flyout'), { devicePixelRatio: 1.5 });
  view.render(state);
  return { win, doc: win.document, log, view };
}

test('withAlpha converts #RRGGBB and passes anything else through', () => {
  assert.equal(withAlpha('#FF2A2A', 0.5), 'rgba(255,42,42,0.5)');
  assert.equal(withAlpha('#22d3ee', FILL_ALPHA), `rgba(34,211,238,${FILL_ALPHA})`);
  assert.equal(withAlpha('red', 0.5), 'red');
  assert.equal(withAlpha('#FF2A2A80', 0.5), '#FF2A2A80', 'eight digits are not #RRGGBB');
  assert.equal(withAlpha('x#FF2A2A', 0.5), 'x#FF2A2A', 'anchored at the start');
  assert.equal(withAlpha(undefined, 0.5), undefined);
});

test('spanLabel reads naturally from seconds to hours', () => {
  assert.equal(spanLabel(0), '');
  assert.equal(spanLabel(NaN), '');
  assert.equal(spanLabel(45), 'last 45 s');
  assert.equal(spanLabel(89), 'last 89 s');
  assert.equal(spanLabel(90), 'last 2 min');
  assert.equal(spanLabel(600), 'last 10 min');
  assert.equal(spanLabel(3600), 'last 1 h');
  assert.equal(spanLabel(4800), 'last 1 h 20 min');
});

test('statsText rounds and shows placeholders when there is no data', () => {
  assert.equal(statsText({ min: 59.6, avg: 70.2, max: 91.5 }, '°'), 'Min 60° · Avg 70° · Max 92°');
  assert.equal(statsText(null, '°'), 'Min -- · Avg -- · Max --');
});

test('runs split at gaps and drop empty stretches', () => {
  assert.deepEqual(runs([[0, 1], [1, 2], null, null, [3, 4]]), [[[0, 1], [1, 2]], [[3, 4]]]);
  assert.deepEqual(runs([null]), []);
  assert.deepEqual(runs([]), []);
});

test('renders live values, band colour, span, axes and legend', () => {
  const { doc } = render(cpuState());
  assert.equal(doc.getElementById('fly-title').textContent, 'CPU');
  assert.equal(doc.getElementById('fly-span').textContent, 'Temperature and fan speed · last 10 min');
  const temp = doc.getElementById('fly-temp');
  assert.equal(temp.className, 'fly-temp num lvl-hot');
  assert.equal(temp.querySelector('b').textContent, '92');
  assert.equal(temp.querySelector('small').textContent, '°C');
  assert.equal(doc.querySelector('#fly-fan b').textContent, '4400');
  assert.equal(doc.getElementById('fly-stats').textContent, 'Min 60° · Avg 77° · Max 92°');
  assert.equal(doc.getElementById('axis-top').textContent, '100°C');
  assert.equal(doc.getElementById('axis-bottom').textContent, '30°C');
  assert.equal(doc.getElementById('axis-fan').textContent, '4840 RPM');
  assert.ok(doc.querySelector('#fly-icon svg'), 'the CPU icon is drawn');
  assert.equal(doc.getElementById('sw-temp').style.background, 'rgb(255, 42, 42)');
});

test('graph: area + line per unbroken run, then the fan line on its own scale', () => {
  const { ctx, log } = recordingContext();
  drawGraph(ctx, 300, 100, cpuState());
  const fills = log.filter((l) => l === 'fill()').length;
  const strokes = log.filter((l) => l === 'stroke()').length;
  assert.equal(fills, 2, 'the gap splits the temperature area in two');
  assert.equal(strokes, 3 + 2 + 2, 'three grid lines, two temperature runs, two fan runs');
  assert.ok(log.includes(`fillStyle = "rgba(255,42,42,${FILL_ALPHA})"`));
  assert.ok(log.includes('strokeStyle = "#22D3EE"'));
  // Newest sample at the right edge: 92 °C of 30–100 is at y = 100 - 62/70*100.
  assert.ok(log.includes(`lineTo(300, ${String(Math.round((100 - (62 / 70) * 100) * 1000) / 1000)})`));
});

test('graph: the area closes straight down from the newest point of each run', () => {
  const { ctx, log } = recordingContext();
  drawGraph(ctx, 300, 100, { ...cpuState(), show_fan: false, capacity: 3, temps: [65, 75, 100] });
  const fill = log.indexOf('fill()');
  const area = log.slice(log.lastIndexOf('beginPath()', fill), fill);
  assert.deepEqual(area, ['beginPath()', 'moveTo(0, 100)', 'lineTo(0, 50)', 'lineTo(150, 35.714)', 'lineTo(300, 0)', 'lineTo(300, 100)', 'closePath()']);
});

test('graph: no fill, no fan trace, and a single point still shows', () => {
  const { ctx, log } = recordingContext();
  drawGraph(ctx, 300, 100, { ...cpuState(), fill: false, show_fan: false, temps: [80], fans: [5000] });
  assert.equal(log.filter((l) => l === 'fill()').length, 0);
  assert.equal(log.filter((l) => l === 'stroke()').length, 3 + 1);
  assert.ok(log.includes('lineTo(301, 28.571)'), 'a one-sample run is drawn as a 1 px dash');
});

test('fan telemetry off hides the fan value, legend and scale', () => {
  const { doc } = render({ ...cpuState(), show_fan: false, fan_rpm: null, fans: [] });
  assert.equal(doc.getElementById('fly-fan').hidden, true);
  assert.equal(doc.getElementById('sw-fan').hidden, true);
  assert.equal(doc.getElementById('legend-fan').hidden, true);
  assert.equal(doc.getElementById('axis-fan').textContent, '');
});

test('switching kind swaps the icon; missing data shows placeholders; no state is ignored', () => {
  const { doc, view } = render(cpuState());
  const cpuIcon = doc.getElementById('fly-icon').innerHTML;
  view.render({ ...cpuState(), kind: 'gpu', title: 'GPU', temp: null, level: 'unknown', fan_rpm: null, stats: null, span_s: 0,
    prefs: { ...cpuState().prefs, unit_symbol: '°F' } });
  assert.notEqual(doc.getElementById('fly-icon').innerHTML, cpuIcon);
  assert.equal(doc.getElementById('fly-title').textContent, 'GPU');
  assert.equal(doc.getElementById('fly-span').textContent, 'Temperature and fan speed');
  assert.equal(doc.querySelector('#fly-temp b').textContent, '--');
  assert.equal(doc.querySelector('#fly-temp small').textContent, '°F');
  assert.equal(doc.getElementById('fly-temp').className, 'fly-temp num lvl-unknown');
  view.render(null);
  assert.equal(doc.getElementById('fly-title').textContent, 'GPU');
});

test('canvas is sized for the device pixel ratio; no 2D context is tolerated', () => {
  const { doc } = render(cpuState(), { size: [312, 122] });
  const c = doc.getElementById('fly-canvas');
  assert.equal(`${c.width}x${c.height}`, '468x183');
  const before = render(cpuState()).doc.getElementById('fly-canvas');
  assert.equal(`${before.width}x${before.height}`, '468x183', 'before layout, the design size is used');
  const win = loadPage('flyout.html');
  win.HTMLCanvasElement.prototype.getContext = () => null;
  createFlyoutView(win.document.getElementById('flyout'), {}).render(cpuState());
  assert.equal(win.document.querySelector('#fly-temp b').textContent, '92');
});

test('asks for a 2D context; keeps the icon when the kind is unchanged or unknown; °C by default', () => {
  const win = loadPage('flyout.html');
  const asked = [];
  win.HTMLCanvasElement.prototype.getContext = (type) => { asked.push(type); return null; };
  const root = win.document.getElementById('flyout');
  const view = createFlyoutView(root, {});
  view.render({ ...cpuState(), prefs: undefined });
  assert.deepEqual(asked, ['2d']);
  assert.equal(root.querySelector('#fly-temp small').textContent, '°C');
  const svg = root.querySelector('#fly-icon svg');
  view.render(cpuState());
  assert.equal(root.querySelector('#fly-icon svg'), svg, 'same kind: the icon is not rebuilt');
  view.render({ ...cpuState(), kind: 'npu' });
  assert.equal(root.querySelector('#fly-icon svg'), svg, 'unknown kind: the icon stays');
});

// Golden (D-20260930-022): markup and draw log of the reference flyout; regenerate only after review.
test('flyout markup and drawing', () => {
  const { doc, log } = render(cpuState(), { size: [312, 122] });
  matchSnapshot('flyout', `${lines(doc.getElementById('flyout').innerHTML)}\n\n${log.join('\n')}\n`);
});
