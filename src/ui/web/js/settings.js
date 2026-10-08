/**
 * @module settings
 * @description Settings page (including the tray graph icons): schema-driven form, fill/read, previews,
 *              and the integration panel (provider status, helper service, diagnostics).
 *
 * @input  Rust `Settings` JSON, integration info, `send(command)`.
 * @output A settings object for `save_settings`; commands for autostart/helper/diagnostics/pinning.
 * @dependencies graph-preview
 */
import { renderPreview } from './graph-preview.js';

const check = (path, label, hint) => ({ path, label, hint, type: 'check' });
const num = (path, label, min, max, step = 1, hint) => ({ path, label, hint, type: 'number', min, max, step });
const select = (path, label, options, hint) => ({ path, label, hint, type: 'select', options });

export const SECTIONS = [
  { legend: 'General', rows: [
    { path: '$autostart', label: 'Start with Windows', type: 'check', hint: 'Adds NitroTray to your sign-in apps. Applied immediately.' },
    check('general.start_minimised', 'Start minimised', 'When started with Windows, stay in the tray instead of opening the popup.'),
    check('general.open_popup_on_tray_click', 'Open popup on tray click', 'Off: a left click opens the tray menu instead.'),
    check('general.close_on_focus_loss', 'Close popup on focus loss'),
    check('general.hotkey_enabled', 'Popup shortcut'),
    { path: 'general.hotkey', label: 'Shortcut keys', type: 'text', hint: 'e.g. Win+Alt+N — needs Ctrl, Alt or Win.' },
  ] },
  { legend: 'Telemetry', rows: [
    select('telemetry.temperature_unit', 'Temperature unit', [['celsius', '°C'], ['fahrenheit', '°F']]),
    select('telemetry.refresh_rate', 'Refresh rate while open', [['fast', 'Fast (0.5 s)'], ['normal', 'Normal (0.5–1 s)'], ['relaxed', 'Relaxed (1–2 s)']]),
    num('telemetry.background_interval_ms', 'Tray icon refresh (ms)', 1000, 5000, 500, 'While the popup is closed. Only temperatures and fans are read.'),
    check('telemetry.gpu_driver_queries', 'GPU driver telemetry (NVML)', 'Utilisation and clocks while a window is open. May briefly wake the discrete GPU.'),
    check('telemetry.fan_telemetry', 'Fan telemetry'),
    num('telemetry.cpu_thresholds.warm_c', 'CPU warm from (°C)', 30, 110), num('telemetry.cpu_thresholds.hot_c', 'CPU red from (°C)', 30, 110),
    num('telemetry.cpu_thresholds.critical_c', 'CPU critical from (°C)', 30, 110),
    num('telemetry.gpu_thresholds.warm_c', 'GPU warm from (°C)', 30, 110), num('telemetry.gpu_thresholds.hot_c', 'GPU red from (°C)', 30, 110),
    num('telemetry.gpu_thresholds.critical_c', 'GPU critical from (°C)', 30, 110),
  ] },
  { legend: 'Tray icons', id: 'icons' },
  { legend: 'Appearance', rows: [
    select('appearance.theme', 'Theme', [['dark', 'Dark'], ['light', 'Light'], ['match_windows', 'Match Windows']]),
    check('appearance.transparency', 'Transparency', 'Translucent popup background (Windows 11 backdrop).'),
    select('appearance.accent_intensity', 'Accent intensity', [['subtle', 'Subtle'], ['normal', 'Normal'], ['vivid', 'Vivid']]),
    check('appearance.reduced_motion', 'Reduced motion', 'No fan-icon rotation or transitions.'),
  ] },
  { legend: 'Startup', rows: [
    check('startup.reapply_last_modes', 'Re-apply last fan and performance modes at startup', 'Off by default: NitroTray never changes hardware modes at boot unless you opt in.'),
  ] },
  { legend: 'Integration', id: 'integration' },
];

export const ICON_FIELDS = [
  check('enabled', 'Show this icon'), check('show_temperature', 'Temperature trace'), check('show_fan', 'Fan-speed trace'),
  { path: 'temperature_color', label: 'Temperature colour', type: 'color' }, { path: 'fan_color', label: 'Fan colour', type: 'color' },
  { path: 'background_color', label: 'Background', type: 'color' }, check('fill', 'Fill under temperature'),
  num('temperature_min_c', 'Graph floor (°C)', 0, 110), num('temperature_max_c', 'Graph ceiling (°C)', 10, 120),
  num('fan_max_rpm', 'Fan full scale (RPM, 0 = auto)', 0, 20000, 100),
];

export function getPath(obj, path) {
  return path.split('.').reduce((o, k) => (o == null ? undefined : o[k]), obj);
}

export function setPath(obj, path, value) {
  const keys = path.split('.');
  let o = obj;
  for (const k of keys.slice(0, -1)) o = o[k] ??= {};
  o[keys.at(-1)] = value;
  return obj;
}

function control(row, prefix) {
  const path = prefix + row.path;
  const attrs = `data-path="${path}" id="f-${path.replace(/[^\w]/g, '-')}"`;
  if (row.type === 'check') return `<input type="checkbox" ${attrs}>`;
  if (row.type === 'select') return `<select ${attrs}>${row.options.map(([v, l]) => `<option value="${v}">${l}</option>`).join('')}</select>`;
  if (row.type === 'number') return `<input type="number" ${attrs} min="${row.min}" max="${row.max}" step="${row.step}">`;
  if (row.type === 'color') return `<input type="color" ${attrs}>`;
  return `<input type="text" ${attrs} maxlength="40">`;
}

function rowHtml(row, prefix) {
  const id = `f-${(prefix + row.path).replace(/[^\w]/g, '-')}`;
  const hint = row.hint ? `<span class="hint">${row.hint}</span>` : '';
  return `<div class="row"><label for="${id}">${row.label}${hint}</label>${control(row, prefix)}</div>`;
}

export function buildForm(form) {
  form.innerHTML = SECTIONS.map((s) => {
    if (s.id === 'icons') {
      const card = (key, name) => `<div class="icon-card" data-icon-card="${key}"><h3><canvas width="32" height="32" data-preview="${key}"></canvas>${name}</h3>${ICON_FIELDS.map((r) => rowHtml(r, `tray_icons.${key}.`)).join('')}</div>`;
      return `<fieldset><legend>${s.legend}</legend>${rowHtml(check('tray_icons.show_app_icon', 'NitroTray app icon'), '')}${rowHtml(check('tray_icons.hover_details', 'Details on hover', 'Hovering the CPU or GPU icon shows its temperature and fan history instead of the plain tooltip.'), '')}<p class="muted">System Informer–style graphs: temperature fill with the fan-speed line on top. Windows may hide new icons in the overflow — use “Show icons on taskbar” below.</p><div class="icon-grid">${card('cpu', 'CPU icon')}${card('gpu', 'GPU icon')}</div></fieldset>`;
    }
    if (s.id === 'integration') return `<fieldset><legend>${s.legend}</legend><dl class="kv" id="integration-kv"></dl>${rowHtml({ path: 'integration.nitrosense_app_id', label: 'NitroSense app id', type: 'text' }, '')}<div class="actions" id="integration-actions"></div></fieldset>`;
    return `<fieldset><legend>${s.legend}</legend>${s.rows.map((r) => rowHtml(r, '')).join('')}</fieldset>`;
  }).join('');
  for (const t of form.querySelectorAll('input[type="text"][data-path="integration.nitrosense_app_id"]')) t.maxLength = 200;
}

export function fillForm(form, settings, autostart) {
  for (const el of form.querySelectorAll('[data-path]')) {
    const v = el.dataset.path === '$autostart' ? autostart : getPath(settings, el.dataset.path);
    if (el.type === 'checkbox') el.checked = Boolean(v);
    // Colour inputs lower-case their value themselves (HTML value sanitisation).
    else if (v !== undefined && v !== null) el.value = String(v);
  }
}

/** Applies form values onto a deep copy of `base`. `$autostart` is excluded (sent separately). */
export function readForm(form, base) {
  const out = structuredClone(base);
  for (const el of form.querySelectorAll('[data-path]')) {
    const path = el.dataset.path;
    if (path.startsWith('$')) continue;
    let v;
    if (el.type === 'checkbox') v = el.checked;
    else if (el.type === 'number') v = Number(el.value);
    else if (el.type === 'color') v = el.value.toUpperCase();
    else v = el.value.trim();
    setPath(out, path, v);
  }
  return out;
}

export function renderPreviews(form, settings, samples) {
  for (const key of ['cpu', 'gpu']) {
    const canvas = form.querySelector(`[data-preview="${key}"]`);
    const s = readForm(form, settings).tray_icons[key];
    renderPreview(canvas, s, samples[`${key}_temp`] || [], samples[`${key}_fan`] || []);
  }
}

const SERVICE_TEXT = { running: 'Running', stopped: 'Stopped', pending: 'Starting/stopping', not_installed: 'Not installed', unknown: 'Unknown' };

export function renderIntegration(form, info, send) {
  const kv = form.querySelector('#integration-kv');
  const actions = form.querySelector('#integration-actions');
  if (!kv || !info) return;
  const rows = [
    ['Hardware provider', info.status_text], ['Provider', info.provider || '—'], ['Detected model', info.model || '—'],
    ['NitroTray helper service', SERVICE_TEXT[info.helper_service] || '—'], ['Acer NitroSense service', SERVICE_TEXT[info.acer_service] || '—'],
    ['Settings folder', info.config_dir || '—'],
  ];
  kv.innerHTML = rows.map(([k, v]) => `<dt>${k}</dt><dd></dd>`).join('');
  kv.querySelectorAll('dd').forEach((dd, i) => { dd.textContent = rows[i][1]; });
  const buttons = [
    info.helper_service !== 'running' && ['install_helper', 'Install helper service…'],
    ['pin_tray_icons', 'Show icons on taskbar'], ['export_diagnostics', 'Export diagnostics'], ['open_nitro_sense', 'Open NitroSense'],
  ].filter(Boolean);
  actions.innerHTML = buttons.map(([cmd, label]) => `<button type="button" class="btn" data-cmd="${cmd}">${label}</button>`).join('');
  actions.onclick = (e) => {
    const b = e.target.closest('[data-cmd]');
    if (b) send({ cmd: b.dataset.cmd });
  };
}
