import { test } from 'node:test';
import assert from 'node:assert/strict';
import { AUTO_HEADROOM, FILL_ALPHA, MIN_AUTO_FAN_RPM, fanScale, renderPreview, yFor } from '../../src/ui/web/js/graph-preview.js';

function fakeCanvas() {
  const ops = [];
  const ctx = {
    fillRect: (...a) => ops.push(['fillRect', a, ctx.fillStyle, ctx.globalAlpha]),
    beginPath: () => ops.push(['beginPath']),
    moveTo: (...a) => ops.push(['moveTo', a]),
    lineTo: (...a) => ops.push(['lineTo', a]),
    stroke: () => ops.push(['stroke', ctx.strokeStyle, ctx.lineWidth]),
    globalAlpha: 1,
  };
  return { canvas: { getContext: () => ctx }, ops };
}

const settings = {
  background_color: '#000000', show_temperature: true, show_fan: true, fill: true,
  temperature_color: '#FF2A2A', fan_color: '#22D3EE', temperature_min_c: 30, temperature_max_c: 100, fan_max_rpm: 0,
};

test('fan scale mirrors the Rust rules', () => {
  assert.equal(fanScale(8000, 9000), 8000);
  assert.equal(fanScale(0, 7000), 7000 * AUTO_HEADROOM);
  assert.equal(fanScale(0, 100), MIN_AUTO_FAN_RPM);
  assert.equal(fanScale(0, Number.NaN), MIN_AUTO_FAN_RPM);
});

test('y mapping', () => {
  assert.equal(yFor(30, 30, 100, 16), 15.5);
  assert.equal(yFor(100, 30, 100, 16), 0.5);
  assert.equal(yFor(5, 5, 5, 16), 15.5);
});

test('renders background, translucent fill and both traces', () => {
  const { canvas, ops } = fakeCanvas();
  assert.equal(renderPreview(canvas, settings, [65, null, 70], [4000, 5000], 16), true);
  assert.equal(canvas.width, 16);
  assert.deepEqual(ops[0], ['fillRect', [0, 0, 16, 16], '#000000', 1]);
  const fills = ops.filter((o) => o[0] === 'fillRect' && o[3] === FILL_ALPHA);
  assert.equal(fills.length, 2, 'one fill column per present temperature sample');
  const strokes = ops.filter((o) => o[0] === 'stroke');
  assert.deepEqual(strokes.map((s) => s[1]), ['#FF2A2A', '#22D3EE']);
  assert.equal(ops.filter((o) => o[0] === 'moveTo').length, 3, 'gap restarts the temperature line');
});

test('respects disabled series and missing canvas support', () => {
  const { canvas, ops } = fakeCanvas();
  renderPreview(canvas, { ...settings, show_fan: false, fill: false }, [65], [4000]);
  assert.equal(ops.filter((o) => o[0] === 'stroke').length, 1);
  assert.equal(ops.filter((o) => o[0] === 'fillRect').length, 1, 'background only, no fill');
  assert.equal(renderPreview({ getContext: undefined }, settings, [], []), false);
  const t = fakeCanvas();
  renderPreview(t.canvas, { ...settings, show_temperature: false }, [65], []);
  assert.equal(t.ops.filter((o) => o[0] === 'stroke').length, 1);
});
