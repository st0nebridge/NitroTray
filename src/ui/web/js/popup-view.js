/**
 * @module popup-view
 * @description Renders the Rust `UiState` into the popup DOM and turns user input into commands.
 *
 * @input  The popup root element, a `send(command)` function, and `UiState` objects.
 * @output DOM updates (text, classes, ARIA) and commands: set_fan_mode, set_performance_mode,
 *         open_*, close_popup, drag_window, dismiss_notice.
 * @dependencies icons, format, segmented
 */
import { hydrate } from './icons.js';
import { fanLabel, fanValue, int, levelClass, pct, ringDash } from './format.js';
import * as segmented from './segmented.js';

const CONNECTED = ['connected', 'simulated'];

export function createPopupView(root, send) {
  const $ = (sel) => root.querySelector(sel);
  const fanSeg = $('#fan-seg');
  const perfSeg = $('#perf-seg');
  let last = null;

  hydrate(root);

  // Disabled choices are sent too: the host answers with an inline explanation.
  segmented.bind(fanSeg, (mode, disabledValue) => send({ cmd: 'set_fan_mode', mode: mode || disabledValue }));
  segmented.bind(perfSeg, (mode, disabledValue) => send({ cmd: 'set_performance_mode', mode: mode || disabledValue }));
  $('#btn-close').addEventListener('click', () => send({ cmd: 'close_popup' }));
  $('#btn-settings').addEventListener('click', () => send({ cmd: 'open_settings' }));
  $('#btn-monitor').addEventListener('click', () => send({ cmd: 'open_monitoring' }));
  $('#btn-quick').addEventListener('click', () => send({ cmd: 'open_nitro_sense' }));
  $('#fan-head').addEventListener('click', () => send({ cmd: 'open_fan_curve' }));
  $('#perf-head').addEventListener('click', () => send({ cmd: 'open_nitro_sense' }));
  for (const id of ['#fan-cpu', '#fan-gpu']) $(id).addEventListener('click', () => send({ cmd: 'open_fan_curve' }));
  $('.notice-x').addEventListener('click', () => send({ cmd: 'dismiss_notice' }));
  $('.hdr').addEventListener('mousedown', (e) => {
    if (e.button === 0 && !e.target.closest('button')) send({ cmd: 'drag_window' });
  });

  function renderFan(el, name, fan) {
    const v = fanValue(fan);
    const value = el.querySelector('.fan-value');
    value.querySelector('b').textContent = v.text;
    value.classList.toggle('na', v.na);
    el.querySelector('.arc').style.strokeDasharray = ringDash(fan?.ring_pct);
    el.classList.toggle('spinning', !v.na && fan.rpm > 0);
    el.setAttribute('aria-label', `${fanLabel(name, fan)} — open fan curve editor`);
  }

  function renderProc(el, p, unitSymbol) {
    const temp = el.querySelector('.temp');
    temp.className = `temp num ${levelClass(p?.level)}`;
    temp.querySelector('b').textContent = int(p?.temp);
    temp.querySelector('small').textContent = unitSymbol;
    el.querySelector('.util').textContent = pct(p?.util);
    el.querySelector('.mhz b').textContent = int(p?.mhz);
  }

  function renderNotice(n) {
    const box = $('#notice');
    box.hidden = !n;
    if (!n) return;
    box.dataset.kind = n.kind;
    box.querySelector('strong').textContent = n.title;
    box.querySelector('.notice-text span').textContent = n.detail || '';
  }

  function render(state) {
    last = state;
    const caps = state.capabilities || {};
    const connected = CONNECTED.includes(state.connection);
    const reason = connected ? 'Not supported on this model' : state.connection_text;
    renderFan($('#fan-cpu'), 'CPU', state.cpu_fan);
    renderFan($('#fan-gpu'), 'GPU', state.gpu_fan);
    renderProc($('#proc-cpu'), state.cpu, state.prefs?.unit_symbol || '°C');
    renderProc($('#proc-gpu'), state.gpu, state.prefs?.unit_symbol || '°C');
    const pending = state.pending || null;
    segmented.render(fanSeg, {
      value: state.fan_mode,
      pending: pending?.kind === 'fan' ? pending.value : null,
      enabled: { auto: connected && caps.auto_fan_mode, max: connected && caps.max_fan_mode, custom: connected && caps.custom_fan_mode },
      reason,
    });
    segmented.render(perfSeg, {
      value: state.performance_mode,
      pending: pending?.kind === 'performance' ? pending.value : null,
      enabled: { quiet: connected && caps.quiet_mode, default: connected && caps.default_mode, performance: connected && caps.performance_mode },
      reason,
    });
    const other = $('#perf-other');
    other.hidden = !state.performance_other;
    other.textContent = state.performance_other ? `${state.performance_other} active` : '';
    $('#btn-settings .badge').hidden = connected;
    $('#btn-settings').title = connected ? 'Settings' : state.connection_text;
    renderNotice(state.notice);
  }

  return { render, get state() { return last; } };
}
