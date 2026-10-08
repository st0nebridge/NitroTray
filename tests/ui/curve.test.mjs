import { test } from 'node:test';
import assert from 'node:assert/strict';
import { errors, key, loadPage } from './helpers/dom.mjs';
import {
  DEFAULT_POINTS, PLOT, RULES, addPoint, clampMove, createCurveEditor, dOf, dutyAt, removePoint, tOf, validate, xOf, yOf,
} from '../../src/ui/web/js/curve.js';

const pts = (arr) => arr.map(([temp_c, duty_pct]) => ({ temp_c, duty_pct }));

test('validation mirrors the Rust CurveError messages', () => {
  assert.equal(validate(DEFAULT_POINTS), null);
  assert.equal(validate(pts([[50, 50]])), 'a curve needs at least 2 points');
  assert.equal(validate(pts(Array.from({ length: 9 }, (_, i) => [30 + i, 30]))), 'a curve allows at most 8 points');
  assert.equal(validate(pts([[19, 30], [60, 50]])), 'point 1 temperature must be 20-100 °C');
  assert.equal(validate(pts([[40, 19], [60, 50]])), 'point 1 duty must be 20-100 %');
  assert.equal(validate(pts([[40, 30], [40.5, 50]])), 'point 2 temperature must be 20-100 °C', 'integers only');
  assert.equal(validate(pts([[50, 30], [50, 40]])), 'point 2 must be hotter than the point before it');
  assert.equal(validate(pts([[40, 60], [60, 50]])), 'point 2 must not lower the fan speed');
});

test('dutyAt mirrors FanCurve::duty_at including the safety override', () => {
  const c = pts([[40, 30], [60, 50], [80, 90]]);
  assert.equal(dutyAt(c, 20), 30);
  assert.equal(dutyAt(c, 50), 40);
  assert.equal(dutyAt(c, 75), 80);
  assert.equal(dutyAt(c, 85), 90);
  assert.equal(dutyAt(c, RULES.SAFETY_C), 100);
  assert.equal(dutyAt(c, Number.NaN), 100);
  assert.equal(dutyAt([], 50), 100);
  assert.equal(dutyAt(pts([[40, 5], [60, 10]]), 50), RULES.MIN_DUTY);
});

test('dragging is clamped between neighbours and to the rule ranges', () => {
  const c = pts([[40, 30], [60, 50], [80, 90]]);
  assert.deepEqual(clampMove(c, 1, 100, 100)[1], { temp_c: 79, duty_pct: 90 });
  assert.deepEqual(clampMove(c, 1, 0, 0)[1], { temp_c: 41, duty_pct: 30 });
  assert.deepEqual(clampMove(c, 0, 0, 0)[0], { temp_c: 20, duty_pct: 20 });
  assert.deepEqual(clampMove(c, 2, 200, 200)[2], { temp_c: 100, duty_pct: 100 });
  assert.equal(c[1].temp_c, 60, 'input not mutated');
  assert.equal(validate(clampMove(c, 1, 65.6, 70.4)), null);
});

test('adding and removing points', () => {
  const c = pts([[40, 30], [80, 90]]);
  assert.deepEqual(addPoint(c)[1], { temp_c: 60, duty_pct: 60 });
  const tight = pts([[40, 30], [41, 40]]);
  assert.equal(addPoint(tight), tight, 'no room');
  const full = pts(Array.from({ length: 8 }, (_, i) => [30 + i * 5, 30 + i]));
  assert.equal(addPoint(full), full);
  const widest = addPoint(pts([[30, 30], [40, 40], [90, 90]]));
  assert.deepEqual(widest[2], { temp_c: 65, duty_pct: 65 });
  assert.equal(removePoint(c, 0), c, 'minimum two points');
  assert.equal(removePoint(pts([[30, 30], [40, 40], [50, 50]]), 1).length, 2);
});

test('plot coordinate transforms invert each other', () => {
  assert.equal(xOf(20), PLOT.x0);
  assert.equal(xOf(100), PLOT.x1);
  assert.equal(yOf(0), PLOT.y0);
  assert.equal(yOf(100), PLOT.y1);
  assert.equal(Math.round(tOf(xOf(55))), 55);
  assert.equal(Math.round(dOf(yOf(42))), 42);
});

test('editor renders points, table and live marker; keyboard and inputs edit', () => {
  const win = loadPage('window.html');
  const card = win.document.querySelector('.curve-card[data-fan="cpu"]');
  const changes = [];
  const ed = createCurveEditor(card, { onChange: (p) => changes.push(p) });
  assert.equal(card.querySelectorAll('.pt').length, DEFAULT_POINTS.length);
  assert.equal(card.querySelectorAll('.points input').length, DEFAULT_POINTS.length * 2);
  ed.setLive(65);
  assert.ok(card.querySelector('.live'));
  ed.setLive(null);
  assert.equal(card.querySelector('.live'), null);
  const pt = card.querySelector('.pt[data-i="1"]');
  pt.focus();
  key(pt, 'ArrowUp');
  assert.equal(ed.getPoints()[1].duty_pct, 36);
  key(card.querySelector('.pt[data-i="1"]'), 'ArrowRight');
  assert.equal(ed.getPoints()[1].temp_c, 51);
  key(card.querySelector('.pt[data-i="1"]'), 'ArrowLeft');
  key(card.querySelector('.pt[data-i="1"]'), 'ArrowDown');
  assert.deepEqual(ed.getPoints()[1], { temp_c: 50, duty_pct: 35 });
  key(card.querySelector('.pt[data-i="0"]'), 'Delete');
  assert.equal(ed.getPoints().length, DEFAULT_POINTS.length - 1);
  key(card.querySelector('.pt[data-i="0"]'), '+');
  assert.equal(ed.getPoints().length, DEFAULT_POINTS.length);
  key(card.querySelector('.pt[data-i="0"]'), 'x');
  const input = card.querySelector('.points input[data-i="0"][data-f="duty_pct"]');
  input.value = '44.4';
  input.dispatchEvent(new win.Event('change', { bubbles: true }));
  assert.equal(ed.getPoints()[0].duty_pct, 44);
  assert.ok(changes.length >= 6);
  ed.setPoints(pts([[30, 30], [90, 100]]));
  assert.equal(card.querySelectorAll('.pt').length, 2);
  ed.reset();
  assert.deepEqual(ed.getPoints(), DEFAULT_POINTS);
});

test('pointer dragging moves a point within constraints', () => {
  const win = loadPage('window.html');
  const card = win.document.querySelector('.curve-card[data-fan="gpu"]');
  const ed = createCurveEditor(card);
  const svg = card.querySelector('svg');
  svg.getBoundingClientRect = () => ({ left: 0, top: 0, width: 420, height: 260 });
  const fire = (type, target, x, y) => target.dispatchEvent(Object.assign(new win.MouseEvent(type, { bubbles: true, clientX: x, clientY: y }), { pointerId: 1 }));
  fire('pointermove', svg, 10, 10);
  fire('pointerdown', card.querySelector('.pt[data-i="2"]'), xOf(60), yOf(50));
  fire('pointermove', svg, xOf(62), yOf(55));
  assert.deepEqual(ed.getPoints()[2], { temp_c: 62, duty_pct: 55 });
  fire('pointerup', svg, 0, 0);
  fire('pointermove', svg, xOf(30), yOf(90));
  assert.deepEqual(ed.getPoints()[2], { temp_c: 62, duty_pct: 55 }, 'released');
  fire('pointerdown', svg, 0, 0);
});

test('validation accepts the exact rule boundaries and rejects just outside them', () => {
  assert.equal(validate(pts([[20, 20], [100, 100]])), null, 'range ends are valid');
  assert.equal(validate(pts([[40, 50], [60, 50]])), null, 'a flat segment is valid');
  assert.equal(validate(pts(Array.from({ length: 8 }, (_, i) => [30 + i, 30]))), null, 'eight points are valid');
  assert.equal(validate(pts([[40, 30], [101, 50]])), 'point 2 temperature must be 20-100 °C');
  assert.equal(validate(pts([[40, 30], [60, 101]])), 'point 2 duty must be 20-100 %');
});

test('adding a point: first widest gap wins ties, a 2 °C gap still splits', () => {
  assert.deepEqual(addPoint(pts([[30, 30], [50, 50], [70, 70]])).map((p) => p.temp_c), [30, 40, 50, 70]);
  assert.deepEqual(addPoint(pts([[30, 30], [60, 60], [70, 70]])).map((p) => p.temp_c), [30, 45, 60, 70]);
  assert.deepEqual(addPoint(pts([[40, 30], [42, 40]]))[1], { temp_c: 41, duty_pct: 35 });
});

test('pointer: nothing moves before a press, point 0 drags, offsets respected, stray targets ignored', () => {
  const win = loadPage('window.html');
  const errs = errors(win);
  const card = win.document.querySelector('.curve-card[data-fan="gpu"]');
  const ed = createCurveEditor(card);
  const svg = card.querySelector('svg');
  svg.setPointerCapture = undefined;
  svg.getBoundingClientRect = () => ({ left: 100, top: 50, width: 840, height: 520 });
  const at = (t, d) => [100 + xOf(t) * 2, 50 + yOf(d) * 2];
  const fire = (type, target, [x, y] = [0, 0]) => target.dispatchEvent(Object.assign(new win.MouseEvent(type, { bubbles: true, clientX: x, clientY: y }), { pointerId: 1 }));
  fire('pointermove', svg, at(45, 60));
  assert.deepEqual(ed.getPoints(), DEFAULT_POINTS, 'no press, no drag');
  fire('pointerdown', svg);
  fire('pointermove', svg, at(45, 60));
  assert.deepEqual(ed.getPoints(), DEFAULT_POINTS, 'pressing the background drags nothing');
  fire('pointerdown', card.querySelector('.pt[data-i="0"]'));
  fire('pointermove', svg, at(30, 30));
  assert.deepEqual(ed.getPoints()[0], { temp_c: 30, duty_pct: 30 }, 'scaled and offset pointer mapping');
  fire('pointerup', svg);
  const released = ed.getPoints();
  fire('pointermove', svg, at(55, 45));
  assert.deepEqual(ed.getPoints(), released, 'after release no point follows the pointer');
  assert.deepEqual(errs, []);
});

test('keyboard and table: Insert adds, arrows redraw and keep focus, other targets ignored', () => {
  const win = loadPage('window.html');
  const errs = errors(win);
  const card = win.document.querySelector('.curve-card[data-fan="cpu"]');
  const ed = createCurveEditor(card);
  const svg = card.querySelector('svg');
  const cx = () => card.querySelector('.pt[data-i="1"]').getAttribute('cx');
  const before = cx();
  card.querySelector('.pt[data-i="1"]').focus();
  const ev = key(card.querySelector('.pt[data-i="1"]'), 'ArrowRight');
  assert.equal(ev.defaultPrevented, true);
  assert.notEqual(cx(), before, 'moved point is redrawn');
  assert.equal(win.document.activeElement, card.querySelector('.pt[data-i="1"]'), 'focus follows the redrawn point');
  key(card.querySelector('.pt[data-i="0"]'), 'Insert');
  assert.equal(ed.getPoints().length, DEFAULT_POINTS.length + 1);
  key(card.querySelector('.pt[data-i="0"]'), 'x');
  assert.equal(ed.getPoints().length, DEFAULT_POINTS.length + 1, 'other keys do nothing');
  key(svg, 'Delete');
  assert.equal(ed.getPoints().length, DEFAULT_POINTS.length + 1, 'keys on the plot background do nothing');

  const input = card.querySelector('.points input[data-i="2"][data-f="temp_c"]');
  const duty = ed.getPoints()[2].duty_pct;
  input.value = '58';
  input.dispatchEvent(new win.Event('change', { bubbles: true }));
  assert.deepEqual(ed.getPoints()[2], { temp_c: 58, duty_pct: duty }, 'the other field is kept');
  card.querySelector('.points').dispatchEvent(new win.Event('change', { bubbles: true }));
  assert.deepEqual(errs, []);
});
