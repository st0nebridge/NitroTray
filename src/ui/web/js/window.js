/**
 * @module window
 * @description Full-window entry point: page navigation, monitoring charts, curve editor, settings form.
 *
 * @input  Host calls: onState(UiState), onWindowData({settings, autostart, integration, history}),
 *         onSample(sample), navigate(page).
 * @output Commands: save_settings, set_autostart, save_fan_curve, install_helper, pin_tray_icons,
 *         export_diagnostics, open_nitro_sense, dismiss_notice, ready.
 * @dependencies bridge, icons, monitor, curve, settings
 */
import { applyPrefs, expose, send } from './bridge.js';
import { hydrate } from './icons.js';
import { createMonitor } from './monitor.js';
import { createCurveEditor, DEFAULT_POINTS, validate } from './curve.js';
import * as settingsView from './settings.js';

const $ = (sel) => document.querySelector(sel);
const PAGES = ['monitoring', 'fan_curve', 'settings'];
let page;
let settings = null;
let lastState = null;
let renderQueued = false;

hydrate(document);
const monitor = createMonitor($('#charts'));
const editors = {
  cpu: createCurveEditor($('.curve-card[data-fan="cpu"]'), { onChange: showCurveError }),
  gpu: createCurveEditor($('.curve-card[data-fan="gpu"]'), { onChange: showCurveError }),
};
const form = $('#settings-form');
settingsView.buildForm(form);

function queueRender() {
  if (renderQueued || page !== 'monitoring') return;
  renderQueued = true;
  requestAnimationFrame(() => { renderQueued = false; monitor.render(); });
}

export function navigate(next) {
  page = PAGES.includes(next) ? next : 'monitoring';
  for (const p of PAGES) $(`#page-${p}`).hidden = p !== page;
  for (const b of document.querySelectorAll('.nav')) {
    if (b.dataset.page === page) b.setAttribute('aria-current', 'page'); else b.removeAttribute('aria-current');
  }
  queueRender();
}

function curveProfile() {
  return { name: $('#curve-name').value.trim() || 'Custom', cpu: { points: editors.cpu.getPoints() }, gpu: { points: editors.gpu.getPoints() } };
}

function showCurveError() {
  const p = curveProfile();
  const err = validate(p.cpu.points) ? `CPU: ${validate(p.cpu.points)}` : validate(p.gpu.points) ? `GPU: ${validate(p.gpu.points)}` : null;
  const box = $('#curve-error');
  box.hidden = !err;
  box.textContent = err || '';
  $('#curve-save').disabled = Boolean(err);
  $('#curve-apply').disabled = Boolean(err) || !(lastState?.capabilities?.custom_fan_mode);
  return !err;
}

function showToast(notice) {
  const t = $('#toast');
  t.hidden = !notice;
  if (!notice) return;
  t.dataset.kind = notice.kind;
  t.innerHTML = '<strong></strong><span></span>';
  t.querySelector('strong').textContent = notice.title;
  t.querySelector('span').textContent = notice.detail || '';
}

function samplesForPreview() {
  return Object.fromEntries(['cpu_temp', 'gpu_temp', 'cpu_fan', 'gpu_fan'].map((k) => [k, monitor.history[k] || []]));
}

expose({
  navigate,
  onState(state) {
    lastState = state;
    applyPrefs(state.prefs);
    monitor.setUnit(state.prefs?.unit);
    const status = $('#side-status');
    const ok = ['connected', 'simulated'].includes(state.connection);
    status.className = `side-status ${ok ? 'ok' : 'bad'}`;
    status.querySelector('span').textContent = state.connection_text;
    const toC = (t) => (state.prefs?.unit === 'fahrenheit' && Number.isFinite(t) ? ((t - 32) * 5) / 9 : t);
    editors.cpu.setLive(toC(state.cpu?.temp));
    editors.gpu.setLive(toC(state.gpu?.temp));
    showCurveError();
    showToast(state.notice);
  },
  onWindowData(data) {
    settings = data.settings;
    settingsView.fillForm(form, settings, data.autostart);
    settingsView.renderIntegration(form, data.integration, send);
    const profile = settings.fan?.custom_profile;
    $('#curve-name').value = profile?.name || 'Balanced custom';
    editors.cpu.setPoints(profile?.cpu?.points || DEFAULT_POINTS);
    editors.gpu.setPoints(profile?.gpu?.points || DEFAULT_POINTS);
    monitor.setHistory(data.history);
    settingsView.renderPreviews(form, settings, samplesForPreview());
    showCurveError();
    queueRender();
  },
  onSample(sample) {
    monitor.push(sample);
    queueRender();
    if (page === 'settings' && settings) settingsView.renderPreviews(form, settings, samplesForPreview());
  },
});

for (const b of document.querySelectorAll('.nav')) b.addEventListener('click', () => navigate(b.dataset.page));
$('#curve-reset').addEventListener('click', () => { editors.cpu.reset(); editors.gpu.reset(); });
$('#curve-save').addEventListener('click', () => showCurveError() && send({ cmd: 'save_fan_curve', profile: curveProfile(), apply: false }));
$('#curve-apply').addEventListener('click', () => showCurveError() && send({ cmd: 'save_fan_curve', profile: curveProfile(), apply: true }));
$('#settings-save').addEventListener('click', () => {
  if (!settings) return;
  send({ cmd: 'save_settings', settings: settingsView.readForm(form, settings) });
});
form.addEventListener('input', (e) => {
  if (e.target.dataset.path === '$autostart') send({ cmd: 'set_autostart', enabled: e.target.checked });
  else if (settings) settingsView.renderPreviews(form, settings, samplesForPreview());
  $('#settings-status').textContent = e.target.dataset.path === '$autostart' ? '' : 'Unsaved changes';
});
$('#toast').addEventListener('click', () => send({ cmd: 'dismiss_notice' }));
window.addEventListener('resize', queueRender);
navigate(new URLSearchParams(location.search).get('page') || location.hash.slice(1));
send({ cmd: 'ready' });
