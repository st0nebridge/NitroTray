import { test } from 'node:test';
import assert from 'node:assert/strict';
import { click, errors, loadPage, readJson, spy } from './helpers/dom.mjs';
import * as view from '../../src/ui/web/js/settings.js';

const defaults = readJson('../fixtures/settings.default.json');

function setup() {
  const win = loadPage('window.html');
  const form = win.document.getElementById('settings-form');
  view.buildForm(form);
  return { win, form };
}

test('every schema path exists in the Rust settings contract', () => {
  const paths = view.SECTIONS.flatMap((s) => s.rows || []).map((r) => r.path).filter((p) => !p.startsWith('$'));
  for (const p of paths) assert.notEqual(view.getPath(defaults, p), undefined, p);
  for (const key of ['cpu', 'gpu']) {
    for (const f of view.ICON_FIELDS) assert.notEqual(view.getPath(defaults, `tray_icons.${key}.${f.path}`), undefined, f.path);
  }
});

test('fill then read round-trips the defaults exactly', () => {
  const { form } = setup();
  view.fillForm(form, defaults, true);
  assert.equal(form.querySelector('[data-path="$autostart"]').checked, true);
  assert.deepEqual(view.readForm(form, defaults), defaults);
});

test('edits are typed correctly and autostart is excluded', () => {
  const { form } = setup();
  view.fillForm(form, defaults, false);
  const set = (path, value) => {
    const el = form.querySelector(`[data-path="${path}"]`);
    if (el.type === 'checkbox') el.checked = value; else el.value = value;
  };
  set('general.hotkey', '  Ctrl+Alt+K ');
  set('telemetry.background_interval_ms', '3500');
  set('telemetry.temperature_unit', 'fahrenheit');
  set('tray_icons.gpu.enabled', false);
  set('tray_icons.cpu.fan_color', '#12ab34');
  set('$autostart', true);
  const out = view.readForm(form, defaults);
  assert.equal(out.general.hotkey, 'Ctrl+Alt+K');
  assert.equal(out.telemetry.background_interval_ms, 3500);
  assert.equal(out.telemetry.temperature_unit, 'fahrenheit');
  assert.equal(out.tray_icons.gpu.enabled, false);
  assert.equal(out.tray_icons.cpu.fan_color, '#12AB34');
  assert.equal(out.$autostart, undefined);
  assert.equal(defaults.general.hotkey, 'Win+Alt+N', 'base not mutated');
});

test('path helpers', () => {
  const o = {};
  view.setPath(o, 'a.b.c', 1);
  assert.deepEqual(o, { a: { b: { c: 1 } } });
  assert.equal(view.getPath(o, 'a.b.c'), 1);
  assert.equal(view.getPath(o, 'a.x.y'), undefined);
});

test('previews render through the canvas when available', () => {
  const { win, form } = setup();
  view.fillForm(form, defaults, false);
  const drawn = [];
  win.HTMLCanvasElement.prototype.getContext = function getContext() {
    drawn.push(this.dataset.preview);
    return new Proxy({}, { get: (_, p) => (['fillStyle', 'strokeStyle', 'lineWidth', 'globalAlpha'].includes(p) ? undefined : () => {}), set: () => true });
  };
  view.renderPreviews(form, defaults, { cpu_temp: [70, 72], cpu_fan: [5000], gpu_temp: [], gpu_fan: [] });
  assert.deepEqual(drawn, ['cpu', 'gpu']);
});

test('integration panel shows status and offers the right actions', () => {
  const { form, win } = setup();
  const errs = errors(win);
  const s = spy();
  view.renderIntegration(form, {
    status_text: 'The NitroTray helper service is not running.', provider: '', model: 'Nitro AN515-58',
    helper_service: 'not_installed', acer_service: 'running', config_dir: 'C:\\cfg',
  }, s.send);
  const dd = [...form.querySelectorAll('#integration-kv dd')].map((d) => d.textContent);
  assert.deepEqual(dd, ['The NitroTray helper service is not running.', '—', 'Nitro AN515-58', 'Not installed', 'Running', 'C:\\cfg']);
  const buttons = [...form.querySelectorAll('#integration-actions [data-cmd]')].map((b) => b.dataset.cmd);
  assert.deepEqual(buttons, ['install_helper', 'pin_tray_icons', 'export_diagnostics', 'open_nitro_sense']);
  click(form.querySelector('[data-cmd="export_diagnostics"]'));
  click(form.querySelector('#integration-actions'));
  assert.deepEqual(s.calls, [{ cmd: 'export_diagnostics' }]);
  assert.deepEqual(errs, [], 'a click between buttons is ignored');
  view.renderIntegration(form, { helper_service: 'running' }, s.send);
  assert.ok(!form.querySelector('[data-cmd="install_helper"]'), 'no install button once running');
  view.renderIntegration(form, null, s.send);
});

test('integration panel labels every row, service state and action', () => {
  const { form } = setup();
  const s = spy();
  const text = (q) => [...form.querySelectorAll(q)].map((e) => e.textContent);
  view.renderIntegration(form, { status_text: 'x', helper_service: 'stopped', acer_service: 'pending' }, s.send);
  assert.deepEqual(text('#integration-kv dt'), ['Hardware provider', 'Provider', 'Detected model', 'NitroTray helper service', 'Acer NitroSense service', 'Settings folder']);
  assert.deepEqual(text('#integration-kv dd').slice(1), ['—', '—', 'Stopped', 'Starting/stopping', '—']);
  assert.deepEqual(text('#integration-actions button'), ['Install helper service…', 'Show icons on taskbar', 'Export diagnostics', 'Open NitroSense']);
  assert.equal(form.querySelector('#integration-kv').textContent, 'Hardware providerxProvider—Detected model—NitroTray helper serviceStoppedAcer NitroSense serviceStarting/stoppingSettings folder—');
  assert.equal(form.querySelector('#integration-actions').textContent, 'Install helper service…Show icons on taskbarExport diagnosticsOpen NitroSense');
  view.renderIntegration(form, { helper_service: 'unknown', acer_service: 'bogus' }, s.send);
  assert.deepEqual(text('#integration-kv dd').slice(3, 5), ['Unknown', '—']);
  view.renderIntegration(form, { helper_service: 'bogus' }, s.send);
  assert.equal(text('#integration-kv dd')[3], '—');
});

test('fill leaves fields without a value untouched and lower-cases colours', () => {
  const { form } = setup();
  const hotkey = form.querySelector('[data-path="general.hotkey"]');
  hotkey.value = 'keep';
  const colour = form.querySelector('[data-path="tray_icons.cpu.fan_color"]');
  view.fillForm(form, { general: { hotkey: null }, tray_icons: { cpu: { fan_color: '#22D3EE' } } }, false);
  assert.equal(hotkey.value, 'keep', 'null keeps the current value');
  assert.equal(colour.value, '#22d3ee');
  view.fillForm(form, {}, false);
  assert.equal(hotkey.value, 'keep', 'missing keeps the current value');
});
