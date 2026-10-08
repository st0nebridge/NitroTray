import { test } from 'node:test';
import assert from 'node:assert/strict';
import { click, errors, loadPage, readJson } from './helpers/dom.mjs';
import { recordingContext } from './helpers/snapshot.mjs';
import { referenceState } from './fixtures/reference-state.js';
import { CHARTS } from '../../src/ui/web/js/monitor.js';
import { DEFAULT_POINTS, xOf } from '../../src/ui/web/js/curve.js';

const defaults = readJson('../fixtures/settings.default.json');
const frame = () => new Promise((r) => setTimeout(r, 5));
const PREVIEW_BG = 'fillRect(0, 0, 32, 32)';

async function boot(url) {
  const win = loadPage('window.html', url);
  const { ctx, log } = recordingContext();
  win.HTMLCanvasElement.prototype.getContext = () => ctx;
  const sent = [];
  win.ipc = { postMessage: (m) => sent.push(JSON.parse(m)) };
  globalThis.ipc = win.ipc;
  const errs = errors(win);
  await import(`../../src/ui/web/js/window.js?t=${Date.now()}-${Math.random()}`);
  const $ = (q) => win.document.querySelector(q);
  const count = (line) => log.filter((l) => l === line).length;
  const renders = () => log.filter((l) => l.startsWith('clearRect')).length / CHARTS.length;
  const visible = () => ['monitoring', 'fan_curve', 'settings'].filter((p) => !$(`#page-${p}`).hidden);
  return { win, $, api: globalThis.nitro, sent, log, count, renders, visible, errs };
}

const input = (win, el) => el.dispatchEvent(new win.Event('input', { bubbles: true }));

test('initial page comes from ?page=, then #hash, else monitoring', async () => {
  assert.deepEqual((await boot('http://nitro.localhost/?page=settings#fan_curve')).visible(), ['settings']);
  assert.deepEqual((await boot('http://nitro.localhost/#fan_curve')).visible(), ['fan_curve']);
  assert.deepEqual((await boot('http://nitro.localhost/?page=nope')).visible(), ['monitoring']);
  const plain = await boot();
  assert.deepEqual(plain.visible(), ['monitoring']);
  assert.ok(plain.$('[data-icon="chart"] svg'), 'navigation icons are hydrated');
});

test('nav buttons switch pages and mark only the current one', async () => {
  const { $, win, visible } = await boot();
  click($('.nav[data-page="fan_curve"]'));
  assert.deepEqual(visible(), ['fan_curve']);
  const current = [...win.document.querySelectorAll('.nav')].filter((b) => b.hasAttribute('aria-current'));
  assert.deepEqual(current.map((b) => b.dataset.page), ['fan_curve']);
});

test('charts render once per frame and only while the monitoring page is shown', async () => {
  const { api, $, win, renders } = await boot();
  await frame();
  assert.equal(renders(), 1, 'first frame');
  api.onSample({ cpu_temp: 70 });
  api.onSample({ cpu_temp: 71 });
  await frame();
  assert.equal(renders(), 2, 'two samples, one frame');
  assert.equal($('[data-key="cpu_temp"]').textContent, '71 °C');
  api.navigate('settings');
  api.onSample({ cpu_temp: 72 });
  win.dispatchEvent(new win.Event('resize'));
  await frame();
  assert.equal(renders(), 2, 'hidden charts are not drawn');
  api.navigate('monitoring');
  await frame();
  assert.equal(renders(), 3, 'showing the page draws the backlog');
  assert.equal($('[data-key="cpu_temp"]').textContent, '72 °C');
  win.dispatchEvent(new win.Event('resize'));
  await frame();
  assert.equal(renders(), 4, 'resize redraws');
});

test('state: status, preferences, units, live markers and notices', async () => {
  const { api, $, win, errs } = await boot();
  api.onWindowData({ settings: defaults, autostart: false, integration: null });
  assert.deepEqual(errs, [], 'window data before any state is fine');

  api.onState({ ...structuredClone(referenceState), connection: 'simulated', connection_text: 'Simulated hardware.' });
  assert.equal($('#side-status').className, 'side-status ok');
  assert.equal($('#side-status span').textContent, 'Simulated hardware.');
  assert.equal(win.document.documentElement.dataset.reducedMotion, 'true');
  assert.equal($('.curve-card[data-fan="cpu"] .live').getAttribute('x1'), String(xOf(85)));
  assert.equal($('#curve-apply').disabled, false, 'custom mode supported and curve valid');

  const f = structuredClone(referenceState);
  f.prefs.unit = 'fahrenheit';
  f.cpu.temp = 185;
  f.gpu.temp = null;
  api.onState(f);
  assert.equal($('.curve-card[data-fan="cpu"] .live').getAttribute('x1'), String(xOf(85)), '185 °F is 85 °C on the curve');
  assert.equal($('.curve-card[data-fan="gpu"] .live'), null, 'no marker without a reading');
  api.onSample({ cpu_temp: 85 });
  await frame();
  assert.equal($('[data-key="cpu_temp"]').textContent, '185 °F');

  api.onState({ ...structuredClone(referenceState), notice: { kind: 'error', title: 'Failed', detail: 'Helper stopped.' } });
  assert.equal($('#toast span').textContent, 'Helper stopped.');
  api.onState({ ...structuredClone(referenceState), notice: { kind: 'info', title: 'Saved' } });
  assert.equal($('#toast').hidden, false);
  assert.equal($('#toast').dataset.kind, 'info');
  assert.equal($('#toast strong').textContent, 'Saved');
  assert.equal($('#toast span').textContent, '');

  api.onState({ ...structuredClone(referenceState), prefs: undefined, capabilities: undefined, cpu: undefined, gpu: undefined });
  assert.equal($('#curve-apply').disabled, true, 'no capabilities, no apply');
  assert.deepEqual(errs, []);
});

test('window data fills the settings form, integration panel, curve editors and history', async () => {
  const { api, $, sent, errs } = await boot();
  api.onState(structuredClone(referenceState));
  await frame();
  api.onWindowData({ settings: defaults, autostart: true, integration: { status_text: 'Connected.', helper_service: 'running' }, history: { cpu_temp: [70, 71, 72] } });
  assert.equal($('[data-path="general.hotkey"]').value, defaults.general.hotkey);
  assert.equal($('[data-path="$autostart"]').checked, true);
  assert.equal($('#integration-kv dd').textContent, 'Connected.');
  assert.equal($('#curve-name').value, 'Balanced custom');
  await frame();
  assert.equal($('[data-key="cpu_temp"]').textContent, '72 °C', 'history drawn');

  const mine = structuredClone(defaults);
  mine.fan.custom_profile = { name: 'Mine', gpu: { points: [{ temp_c: 40, duty_pct: 30 }, { temp_c: 80, duty_pct: 90 }] } };
  api.onWindowData({ settings: mine, autostart: false, integration: null });
  assert.equal($('#curve-name').value, 'Mine');
  api.navigate('fan_curve');
  click($('#curve-save'));
  assert.deepEqual(sent.at(-1).profile.cpu.points, DEFAULT_POINTS, 'missing CPU curve falls back to the default');
  assert.equal(sent.at(-1).profile.gpu.points.length, 2);
  mine.fan.custom_profile = { name: 'CPU only', cpu: { points: [{ temp_c: 45, duty_pct: 30 }, { temp_c: 85, duty_pct: 95 }] } };
  api.onWindowData({ settings: mine, autostart: false, integration: null });
  click($('#curve-save'));
  assert.equal(sent.at(-1).profile.cpu.points.length, 2);
  assert.deepEqual(sent.at(-1).profile.gpu.points, DEFAULT_POINTS, 'missing GPU curve falls back to the default');

  api.onWindowData({ settings: {}, autostart: false, integration: null });
  assert.deepEqual(errs, [], 'settings without a fan section');
});

test('curve profile name, errors per fan, reset, and edits flagged as they happen', async () => {
  const { api, $, win, sent } = await boot();
  api.onState(structuredClone(referenceState));
  api.onWindowData({ settings: defaults, autostart: false, integration: null });
  api.navigate('fan_curve');
  assert.equal($('#curve-error').hidden, true);
  assert.equal($('#curve-error').textContent, '');

  $('#curve-name').value = '   ';
  click($('#curve-save'));
  assert.equal(sent.at(-1).profile.name, 'Custom');
  $('#curve-name').value = '  Night  ';
  click($('#curve-apply'));
  assert.equal(sent.at(-1).cmd, 'save_fan_curve');
  assert.equal(sent.at(-1).profile.name, 'Night');
  assert.equal(sent.at(-1).apply, true);

  const edit = (fan, field, value) => {
    const el = $(`.curve-card[data-fan="${fan}"] .points input[data-i="1"][data-f="${field}"]`);
    el.value = String(value);
    el.dispatchEvent(new win.Event('change', { bubbles: true }));
  };
  edit('gpu', 'temp_c', 10);
  assert.equal($('#curve-error').hidden, false);
  assert.equal($('#curve-error').textContent, 'GPU: point 2 temperature must be 20-100 °C');
  assert.equal($('#curve-save').disabled, true);
  edit('cpu', 'duty_pct', 5);
  assert.equal($('#curve-error').textContent, 'CPU: point 2 duty must be 20-100 %');

  click($('#curve-reset'));
  assert.equal($('#curve-error').hidden, true);
  click($('#curve-save'));
  assert.deepEqual(sent.at(-1).profile.cpu.points, DEFAULT_POINTS);
  assert.deepEqual(sent.at(-1).profile.gpu.points, DEFAULT_POINTS);
});

test('settings page: previews follow samples and edits; autostart applies at once', async () => {
  const { api, $, win, sent, log, count, errs } = await boot('http://nitro.localhost/?page=settings');
  click($('#settings-save'));
  input(win, $('[data-path="general.hotkey"]'));
  api.onSample({ cpu_temp: 70 });
  assert.equal(sent.filter((c) => c.cmd === 'save_settings').length, 0, 'nothing to save before data arrives');
  assert.equal(count(PREVIEW_BG), 0, 'no previews before data arrives');
  assert.deepEqual(errs, []);

  api.onWindowData({ settings: defaults, autostart: false, integration: null });
  assert.equal(count(PREVIEW_BG), 2, 'CPU and GPU previews');
  log.length = 0;
  api.onSample({ cpu_temp: 72, gpu_temp: 60, cpu_fan: 5000, gpu_fan: 4000 });
  assert.equal(count(PREVIEW_BG), 2);
  const newest = log.filter((l) => /^(moveTo|lineTo)\(31\.5, /.test(l));
  assert.equal(newest.length, 4, `each preview series ends at the newest sample: ${newest}`);
  assert.ok(log.includes('moveTo(30.5, 13.786)') && log.includes('lineTo(31.5, 12.9)'), 'CPU temperature keeps its earlier sample (70 → 72 °C)');

  log.length = 0;
  input(win, $('[data-path="general.hotkey"]'));
  assert.equal(count(PREVIEW_BG), 2, 'edits refresh the previews');
  assert.equal($('#settings-status').textContent, 'Unsaved changes');
  const auto = $('[data-path="$autostart"]');
  auto.checked = true;
  input(win, auto);
  assert.deepEqual(sent.at(-1), { cmd: 'set_autostart', enabled: true });
  assert.equal($('#settings-status').textContent, '');

  api.navigate('monitoring');
  log.length = 0;
  api.onSample({ cpu_temp: 73 });
  assert.equal(count(PREVIEW_BG), 0, 'previews only while the settings page is shown');
});
