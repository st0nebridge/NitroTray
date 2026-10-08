import { test } from 'node:test';
import assert from 'node:assert/strict';
import { DASH, fanLabel, fanValue, int, levelClass, pct, ringDash } from '../../src/ui/web/js/format.js';

test('numbers round, missing values show dashes', () => {
  assert.equal(int(7316.6), '7317');
  assert.equal(int(null), DASH);
  assert.equal(int(Number.NaN), '--');
  assert.equal(pct(20.6), '21%');
  assert.equal(pct(undefined), '--%');
});

test('temperature level classes', () => {
  for (const l of ['normal', 'warm', 'hot', 'critical']) assert.equal(levelClass(l), `lvl-${l}`);
  assert.equal(levelClass('unknown'), 'lvl-unknown');
  assert.equal(levelClass(undefined), 'lvl-unknown');
  assert.equal(levelClass('<script>'), 'lvl-unknown');
});

test('ring dash arrays never imply a percentage that is not known', () => {
  assert.equal(ringDash(60), '60.0 40.0');
  assert.equal(ringDash(100), '100.0 0.0');
  assert.equal(ringDash(150), '100.0 0.0');
  assert.equal(ringDash(0), '0 100');
  assert.equal(ringDash(null), '0 100');
  assert.equal(ringDash(-5), '0 100');
});

test('fan cell values and labels', () => {
  assert.deepEqual(fanValue({ state: 'ok', rpm: 7317 }), { text: '7317', na: false });
  assert.deepEqual(fanValue({ state: 'ok', rpm: null }), { text: '--', na: true });
  assert.deepEqual(fanValue({ state: 'unavailable' }), { text: 'Unavailable', na: true });
  assert.deepEqual(fanValue({ state: 'disabled' }), { text: 'Off', na: true });
  assert.deepEqual(fanValue(undefined), { text: 'Unavailable', na: true });
  assert.equal(fanLabel('CPU', { state: 'ok', rpm: 7317 }), 'CPU fan 7317 RPM');
  assert.equal(fanLabel('GPU', { state: 'unavailable' }), 'GPU fan unavailable');
});
