import { test } from 'node:test';
import assert from 'node:assert/strict';
import { click, key, loadPage, readJson } from './helpers/dom.mjs';
import { helperMissingState, referenceState } from './fixtures/reference-state.js';

const defaults = readJson('../fixtures/settings.default.json');
const tick = () => new Promise((r) => setTimeout(r, 5));

function capture(win) {
  const sent = [];
  win.ipc = { postMessage: (m) => sent.push(JSON.parse(m)) };
  globalThis.ipc = win.ipc;
  return sent;
}

test('popup entry: announces ready, renders pushed state, Escape closes, focusFirst', async () => {
  const win = loadPage('popup.html');
  const sent = capture(win);
  await import(`../../src/ui/web/js/popup.js?t=${Date.now()}`);
  assert.deepEqual(sent[0], { cmd: 'ready' });
  win.nitro = globalThis.nitro;
  globalThis.nitro.onState(structuredClone(referenceState));
  assert.equal(win.document.querySelector('#fan-cpu .fan-value b').textContent, '7317');
  assert.equal(win.document.documentElement.dataset.reducedMotion, 'true');
  globalThis.nitro.focusFirst();
  assert.equal(win.document.activeElement.dataset.value, 'auto');
  key(win.document.body, 'Escape');
  assert.deepEqual(sent.at(-1), { cmd: 'close_popup' });
  const ev = new win.MouseEvent('contextmenu', { bubbles: true, cancelable: true });
  win.document.body.dispatchEvent(ev);
  assert.equal(ev.defaultPrevented, true);
  key(win.document.body, 'a');
  assert.deepEqual(sent.at(-1), { cmd: 'close_popup' });
});

test('window entry: navigation, data, samples, curve and settings commands', async () => {
  const win = loadPage('window.html');
  win.HTMLCanvasElement.prototype.getContext = () => null;
  const sent = capture(win);
  await import(`../../src/ui/web/js/window.js?t=${Date.now()}`);
  const api = globalThis.nitro;
  const $ = (q) => win.document.querySelector(q);
  assert.deepEqual(sent[0], { cmd: 'ready' });
  assert.equal($('#page-monitoring').hidden, false);
  api.navigate('settings');
  assert.equal($('#page-settings').hidden, false);
  assert.equal($('[data-page="settings"]').getAttribute('aria-current'), 'page');
  api.navigate('bogus');
  assert.equal($('#page-monitoring').hidden, false);

  api.onState(structuredClone(referenceState));
  assert.equal($('#side-status').className, 'side-status ok');
  api.onWindowData({ settings: defaults, autostart: false, integration: { helper_service: 'running', status_text: 'ok' }, history: { cpu_temp: [70, 71] } });
  api.onSample({ cpu_temp: 72 });
  await tick();
  assert.equal($('#curve-name').value, 'Balanced custom');

  click($('[data-page="fan_curve"]'));
  click($('#curve-save'));
  assert.equal(sent.at(-1).cmd, 'save_fan_curve');
  assert.equal(sent.at(-1).apply, false);
  click($('#curve-apply'));
  assert.equal(sent.at(-1).apply, true);
  click($('#curve-reset'));

  api.onState(structuredClone({ ...helperMissingState, prefs: { ...helperMissingState.prefs, unit: 'fahrenheit' } }));
  assert.equal($('#side-status').className, 'side-status bad');
  assert.equal($('#curve-apply').disabled, true, 'cannot apply without custom-mode support');
  assert.equal($('#toast').hidden, false);
  click($('#toast'));
  assert.deepEqual(sent.at(-1), { cmd: 'dismiss_notice' });

  const withCurve = structuredClone(defaults);
  withCurve.fan.custom_profile = { name: 'Mine', cpu: { points: [{ temp_c: 50, duty_pct: 50 }] }, gpu: { points: [{ temp_c: 40, duty_pct: 30 }, { temp_c: 80, duty_pct: 90 }] } };
  api.onWindowData({ settings: withCurve, autostart: true, integration: null });
  assert.equal($('#curve-error').hidden, false, 'invalid stored CPU curve is flagged');
  assert.match($('#curve-error').textContent, /^CPU: /);
  const before = sent.length;
  click($('#curve-save'));
  assert.equal(sent.length, before, 'invalid curves are not sent');

  api.navigate('settings');
  const auto = $('[data-path="$autostart"]');
  auto.checked = false;
  auto.dispatchEvent(new win.Event('input', { bubbles: true }));
  assert.deepEqual(sent.at(-1), { cmd: 'set_autostart', enabled: false });
  const hk = $('[data-path="general.hotkey"]');
  hk.value = 'Ctrl+Alt+H';
  hk.dispatchEvent(new win.Event('input', { bubbles: true }));
  assert.equal($('#settings-status').textContent, 'Unsaved changes');
  api.onSample({ cpu_temp: 73 });
  click($('#settings-save'));
  assert.equal(sent.at(-1).cmd, 'save_settings');
  assert.equal(sent.at(-1).settings.general.hotkey, 'Ctrl+Alt+H');
  win.dispatchEvent(new win.Event('resize'));
  api.onState(structuredClone({ ...referenceState, notice: null }));
  assert.equal($('#toast').hidden, true);
});

test('popup entry: focusFirst falls back to Close, only Escape closes', async () => {
  const win = loadPage('popup.html');
  const sent = capture(win);
  await import(`../../src/ui/web/js/popup.js?t=${Date.now()}-b`);
  const focused = [];
  const orig = win.HTMLElement.prototype.focus;
  win.HTMLElement.prototype.focus = function focus(opts) { focused.push([this.id || this.dataset.value, opts]); orig.call(this, opts); };
  globalThis.nitro.focusFirst();
  assert.deepEqual(focused.at(-1), ['btn-close', { preventScroll: true }], 'nothing rendered yet: Close gets focus');
  globalThis.nitro.onState(structuredClone(referenceState));
  assert.equal(win.document.documentElement.dataset.accent, 'normal', 'preferences applied');
  globalThis.nitro.focusFirst();
  assert.deepEqual(focused.at(-1), ['auto', { preventScroll: true }], 'then the selected fan mode');
  assert.equal(win.document.activeElement.dataset.value, 'auto');
  for (const b of win.document.querySelectorAll('#fan-seg button')) b.tabIndex = -1;
  win.document.getElementById('btn-close').remove();
  globalThis.nitro.focusFirst();
  assert.equal(focused.length, 2, 'no target, no focus, no error');

  const n = sent.length;
  key(win.document.body, 'a');
  assert.equal(sent.length, n, 'other keys do nothing');
  const esc = key(win.document.body, 'Escape');
  assert.equal(esc.defaultPrevented, true);
  assert.deepEqual(sent.slice(n), [{ cmd: 'close_popup' }]);
});

test('flyout entry: announces ready, renders pushed state, blocks the context menu', async () => {
  const win = loadPage('flyout.html');
  win.HTMLCanvasElement.prototype.getContext = () => null;
  const sent = capture(win);
  await import(`../../src/ui/web/js/flyout.js?t=${Date.now()}`);
  assert.deepEqual(sent, [{ cmd: 'ready' }]);
  globalThis.nitro.onFlyout({
    kind: 'gpu', title: 'GPU', temp: 71, level: 'warm', fan_rpm: 3700, temps: [70, 71], fans: [3600, 3700],
    capacity: 300, temp_range: [30, 100], fan_max: 4070, temp_color: '#FFA53A', fan_color: '#3DDC84',
    fill: true, show_fan: true, stats: { min: 70, avg: 70.5, max: 71 }, span_s: 2,
    prefs: { theme: 'light', accent: 'normal', reduced_motion: false, transparency: false, unit_symbol: '°C', unit: 'celsius' },
  });
  assert.equal(win.document.querySelector('#fly-temp b').textContent, '71');
  assert.equal(win.document.documentElement.dataset.theme, 'light');
  globalThis.nitro.onFlyout(null);
  const ev = new win.MouseEvent('contextmenu', { bubbles: true, cancelable: true });
  win.document.body.dispatchEvent(ev);
  assert.equal(ev.defaultPrevented, true);
  assert.equal(sent.length, 1, 'the flyout never sends commands besides ready');
});
