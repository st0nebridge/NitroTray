import { test } from 'node:test';
import assert from 'node:assert/strict';
import { loadPage } from './helpers/dom.mjs';
import { CAPACITY, CHARTS, createMonitor, formatValue, niceMax, toF, toPoints, yRange } from '../../src/ui/web/js/monitor.js';

test('niceMax rounds up to a 1/2/2.5/5 step', () => {
  assert.equal(niceMax(7317), 10000);
  assert.equal(niceMax(2100), 2500);
  assert.equal(niceMax(4100), 5000);
  assert.equal(niceMax(180), 200);
  assert.equal(niceMax(100), 100);
  assert.equal(niceMax(0), 1);
  assert.equal(niceMax(Number.NaN), 1);
});

test('y ranges: fixed, auto, and Fahrenheit for temperatures', () => {
  const temps = CHARTS.find((c) => c.id === 'temps');
  const fans = CHARTS.find((c) => c.id === 'fans');
  assert.deepEqual(yRange(temps, [[50]]), [30, 100]);
  assert.deepEqual(yRange(temps, [[50]], true), [86, 212]);
  assert.deepEqual(yRange(fans, [[7000, null], [7692]]), [0, 10000]);
  assert.deepEqual(yRange(fans, [[]]), [0, 1]);
  assert.deepEqual(yRange(fans, [[7000, Number.NaN]]), [0, 10000], 'non-finite values are ignored');
  assert.equal(toF(100), 212);
});

test('points: newest at the right edge, gaps preserved, clamped to range', () => {
  const pts = toPoints([0, null, 100, 200], 300, 100, [0, 100], 4);
  assert.deepEqual(pts[0], [0, 100]);
  assert.equal(pts[1], null);
  assert.deepEqual(pts[2], [200, 0]);
  assert.deepEqual(pts[3], [300, 0], 'clamped');
  const short = toPoints([50], 300, 100, [0, 100], 4);
  assert.deepEqual(short[0], [300, 50], 'right-aligned');
  assert.deepEqual(toPoints([5], 10, 10, [5, 5], 1)[0], [0, 10], 'zero span and capacity 1 are safe');
});

test('formatting per unit', () => {
  assert.equal(formatValue(95.46, 'W'), '95.5');
  assert.equal(formatValue(7316.7, 'RPM'), '7317');
  assert.equal(formatValue(null, '%'), '--');
});

test('monitor builds cards, keeps bounded history, and updates legends', () => {
  const win = loadPage('window.html');
  const box = win.document.getElementById('charts');
  const m = createMonitor(box, win.document);
  assert.equal(box.querySelectorAll('.chart-card').length, CHARTS.length);
  m.setHistory({ cpu_temp: Array(CAPACITY + 20).fill(80), gpu_fan: [7000], timestamps: [1], bogus: 'x' });
  assert.equal(m.history.cpu_temp.length, CAPACITY);
  m.push({ timestamp: 5, cpu_temp: 85, gpu_temp: 'NaN', cpu_fan: 7317 });
  assert.equal(m.history.cpu_temp.at(-1), 85);
  assert.equal(m.history.gpu_temp.at(-1), null);
  assert.equal(m.history.timestamp, undefined);
  for (let i = 0; i < CAPACITY + 5; i += 1) m.push({ cpu_fan: i });
  assert.equal(m.history.cpu_fan.length, CAPACITY);
  m.render();
  assert.equal(box.querySelector('[data-key="cpu_temp"]').textContent, '85 °C');
  m.setUnit('fahrenheit');
  m.render();
  assert.equal(box.querySelector('[data-key="cpu_temp"]').textContent, '185 °F');
  assert.equal(box.querySelector('[data-key="gpu_power"]').textContent, '-- W');
  m.setHistory(null);
  m.push(null);
});

test('drawing uses the canvas when available', () => {
  const win = loadPage('window.html');
  const box = win.document.getElementById('charts');
  const ops = [];
  const ctx = new Proxy({}, { get: (_, p) => (typeof p === 'string' && !['lineWidth', 'strokeStyle', 'fillStyle', 'font'].includes(p) ? (...a) => ops.push([p, a]) : undefined), set: () => true });
  win.HTMLCanvasElement.prototype.getContext = () => ctx;
  const m = createMonitor(box, win.document);
  m.setHistory({ cpu_temp: [70, null, 72], gpu_temp: [60, 61, 62] });
  m.render();
  const names = ops.map(([n]) => n);
  assert.ok(names.includes('setTransform') && names.includes('fillText') && names.includes('lineTo'));
  assert.ok(names.filter((n) => n === 'moveTo').length > CHARTS.length, 'gaps restart the pen');
});

test('history ignores non-arrays and starts new series cleanly', () => {
  const win = loadPage('window.html');
  const m = createMonitor(win.document.getElementById('charts'), win.document);
  m.setHistory({ bogus: 'x', gpu_util: [1, 2] });
  assert.equal(m.history.bogus, undefined);
  m.push({ gpu_power: 42.5 });
  assert.deepEqual(m.history.gpu_power, [42.5]);
});
