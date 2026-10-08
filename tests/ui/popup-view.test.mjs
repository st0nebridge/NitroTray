import { test } from 'node:test';
import assert from 'node:assert/strict';
import { click, loadPage, spy } from './helpers/dom.mjs';
import { createPopupView } from '../../src/ui/web/js/popup-view.js';
import { helperMissingState, referenceState } from './fixtures/reference-state.js';

function setup(state = referenceState) {
  const win = loadPage();
  const doc = win.document;
  const s = spy();
  const view = createPopupView(doc.getElementById('popup'), s.send);
  view.render(structuredClone(state));
  return { win, doc, s, view, $: (q) => doc.querySelector(q) };
}

test('renders the reference values in the reference places', () => {
  const { $, view } = setup();
  assert.equal($('#fan-cpu .fan-value b').textContent, '7317');
  assert.equal($('#fan-gpu .fan-value b').textContent, '7692');
  assert.equal($('#fan-cpu .arc').style.strokeDasharray, '60.0 40.0');
  assert.ok($('#fan-cpu').classList.contains('spinning'));
  assert.equal($('#proc-cpu .temp b').textContent, '85');
  assert.equal($('#proc-cpu .temp').className, 'temp num lvl-hot');
  assert.equal($('#proc-gpu .temp').className, 'temp num lvl-warm');
  assert.equal($('#proc-cpu .temp small').textContent, '°C');
  assert.equal($('#proc-cpu .util').textContent, '21%');
  assert.equal($('#proc-cpu .mhz b').textContent, '3947');
  assert.equal($('#proc-gpu .util').textContent, '93%');
  assert.equal($('#proc-gpu .mhz b').textContent, '2565');
  assert.equal($('#fan-seg [data-value="auto"]').getAttribute('aria-checked'), 'true');
  assert.equal($('#perf-seg [data-value="default"]').getAttribute('aria-checked'), 'true');
  assert.equal($('#notice').hidden, true);
  assert.equal($('#btn-settings .badge').hidden, true);
  assert.equal($('#perf-other').hidden, true);
  assert.equal($('#fan-cpu').getAttribute('aria-label'), 'CPU fan 7317 RPM — open fan curve editor');
  assert.equal(view.state.cpu.temp, 85);
});

test('helper missing: read-only degradation with explanations', () => {
  const { $ } = setup(helperMissingState);
  assert.equal($('#fan-cpu .fan-value b').textContent, 'Unavailable');
  assert.ok($('#fan-cpu .fan-value').classList.contains('na'));
  assert.ok(!$('#fan-cpu').classList.contains('spinning'));
  assert.equal($('#fan-cpu .arc').style.strokeDasharray, '0 100');
  assert.equal($('#proc-cpu .temp b').textContent, '--');
  assert.equal($('#proc-cpu .util').textContent, '18%');
  for (const b of document.querySelectorAll('.seg button')) {
    assert.equal(b.getAttribute('aria-disabled'), 'true');
    assert.match(b.title, /helper service is not running/);
  }
  assert.equal($('#btn-settings .badge').hidden, false);
  assert.match($('#btn-settings').title, /not running/);
  assert.equal($('#notice').hidden, false);
  assert.equal($('#notice strong').textContent, 'Could not switch to Performance mode.');
  assert.equal($('#notice .notice-text span').textContent, 'NitroSense service did not accept the request.');
  assert.equal($('#notice').dataset.kind, 'error');
});

test('unsupported modes on a connected machine say so', () => {
  const state = structuredClone(referenceState);
  state.capabilities.custom_fan_mode = false;
  state.performance_mode = null;
  state.performance_other = 'Turbo';
  state.pending = { kind: 'fan', value: 'max' };
  state.prefs.unit_symbol = '°F';
  state.notice = { kind: 'info', title: 'Saved', detail: '' };
  const { $ } = setup(state);
  assert.equal($('#fan-seg [data-value="custom"]').title, 'Not supported on this model');
  assert.equal($('#fan-seg [data-value="max"]').getAttribute('aria-busy'), 'true');
  assert.equal($('#perf-other').hidden, false);
  assert.equal($('#perf-other').textContent, 'Turbo active');
  assert.equal($('#proc-gpu .temp small').textContent, '°F');
  assert.equal($('#notice .notice-text span').textContent, '');
});

test('user actions send the right commands', () => {
  const { $, s } = setup();
  click($('#fan-seg [data-value="max"]'));
  click($('#perf-seg [data-value="quiet"]'));
  click($('#btn-close'));
  click($('#btn-settings'));
  click($('#btn-monitor'));
  click($('#btn-quick'));
  click($('#fan-head'));
  click($('#perf-head'));
  click($('#fan-gpu'));
  click($('.notice-x'));
  assert.deepEqual(s.calls.map((c) => c.cmd), [
    'set_fan_mode', 'set_performance_mode', 'close_popup', 'open_settings', 'open_monitoring',
    'open_nitro_sense', 'open_fan_curve', 'open_nitro_sense', 'open_fan_curve', 'dismiss_notice',
  ]);
  assert.equal(s.calls[0].mode, 'max');
  assert.equal(s.calls[1].mode, 'quiet');
});

test('clicking a disabled option still asks the host (which explains why)', () => {
  const { $, s } = setup(helperMissingState);
  click($('#fan-seg [data-value="max"]'));
  click($('#perf-seg [data-value="performance"]'));
  assert.deepEqual(s.calls, [{ cmd: 'set_fan_mode', mode: 'max' }, { cmd: 'set_performance_mode', mode: 'performance' }]);
});

test('header drag starts only from non-button areas with the primary button', () => {
  const { $, s, win } = setup();
  const down = (el, button = 0) => el.dispatchEvent(new win.MouseEvent('mousedown', { bubbles: true, button }));
  down($('.title'));
  down($('.title'), 2);
  down($('#btn-close'));
  assert.deepEqual(s.calls, [{ cmd: 'drag_window' }]);
});

test('controls are enabled only when connected and supported; simulated counts as connected', () => {
  const { $ } = setup();
  for (const b of document.querySelectorAll('.seg button')) assert.equal(b.getAttribute('aria-disabled'), 'false', b.dataset.value);
  assert.equal($('#btn-settings').title, 'Settings');
  assert.equal($('#perf-other').textContent, '');
  assert.equal($('#fan-gpu').getAttribute('aria-label'), 'GPU fan 7692 RPM — open fan curve editor');
  assert.ok($('#fan-cpu [data-icon] svg'), 'icons hydrated');

  const offline = { ...structuredClone(referenceState), connection: 'access_denied', connection_text: 'Needs the helper.' };
  const off = setup(offline);
  for (const b of document.querySelectorAll('.seg button')) {
    assert.equal(b.getAttribute('aria-disabled'), 'true', `${b.dataset.value}: supported but not connected`);
    assert.equal(b.title, 'Needs the helper.');
  }
  assert.equal(off.$('#btn-settings').title, 'Needs the helper.');

  setup({ ...structuredClone(referenceState), connection: 'simulated' });
  for (const b of document.querySelectorAll('.seg button')) assert.equal(b.getAttribute('aria-disabled'), 'false');
});

test('pending performance change, idle fans and missing sections', () => {
  const state = structuredClone(referenceState);
  state.pending = { kind: 'performance', value: 'quiet' };
  state.cpu_fan = { state: 'ok', rpm: 0, ring_pct: 0 };
  state.gpu_fan = { state: 'unavailable', rpm: 5000, ring_pct: 50 };
  const { $ } = setup(state);
  assert.equal($('#perf-seg [data-value="quiet"]').getAttribute('aria-busy'), 'true');
  assert.equal($('#fan-seg [aria-busy]'), null, 'a performance change does not mark the fan control');
  assert.ok(!$('#fan-cpu').classList.contains('spinning'), '0 RPM does not spin');
  assert.ok(!$('#fan-gpu').classList.contains('spinning'), 'unavailable does not spin');

  const bare = { ...structuredClone(referenceState), cpu: undefined, gpu: undefined, cpu_fan: undefined, gpu_fan: undefined, prefs: undefined };
  const b = setup(bare);
  assert.equal(b.$('#proc-cpu .temp b').textContent, '--');
  assert.equal(b.$('#proc-cpu .temp small').textContent, '°C', 'default unit symbol');
  assert.equal(b.$('#proc-gpu .temp small').textContent, '°C');
  assert.equal(b.$('#fan-cpu .fan-value b').textContent, 'Unavailable');
});

test('header drag: each gesture checked on its own', () => {
  const { $, s, win } = setup();
  const down = (el, button = 0) => el.dispatchEvent(new win.MouseEvent('mousedown', { bubbles: true, button }));
  down($('.title'), 2);
  assert.equal(s.calls.length, 0, 'secondary button');
  down($('#btn-close'));
  assert.equal(s.calls.length, 0, 'buttons keep their own click');
  down($('.title'));
  assert.deepEqual(s.calls, [{ cmd: 'drag_window' }]);
});
