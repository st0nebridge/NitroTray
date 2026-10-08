/**
 * @module icons
 * @description One purpose-built line-icon family: fan, CPU, GPU, modes, actions.
 *
 * @output Functions returning inline SVG markup strings (currentColor, no ids, safe to repeat), and
 *         `hydrate(root)` which fills every `[data-icon]` placeholder.
 * @dependencies none
 */

const svg = (viewBox, body, extra = '') =>
  `<svg viewBox="${viewBox}" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" ${extra}>${body}</svg>`;

const rotations = (n, body) =>
  Array.from({ length: n }, (_, i) => `<g transform="rotate(${(360 / n) * i} 12 12)">${body}</g>`).join('');

/** Swirl fan: hub plus eight curved double-stroke blades. */
export const fan = (sw = 1.35) =>
  svg(
    '0 0 24 24',
    rotations(8, `<path d="M12 9.1C11.3 6.8 12.3 4.3 14.8 3.3"/><path d="M13.3 9.5C13.9 7.5 15.6 6.1 17.7 5.9" opacity=".75"/>`) +
      `<circle cx="12" cy="12" r="2.5" fill="currentColor" stroke="none"/>`,
    `stroke-width="${sw}"`,
  );

/** Max: a dense turbine disc. */
export const fanMax = () =>
  svg(
    '0 0 24 24',
    `<circle cx="12" cy="12" r="9.6" fill="currentColor" fill-opacity=".22" stroke-width="1.2"/>` +
      rotations(14, `<path d="M12 9.4C11.2 7.2 11.8 4.5 13.7 2.6"/>`) +
      `<circle cx="12" cy="12" r="2.8" fill="currentColor" stroke="none"/>`,
    'stroke-width="1.15"',
  );

/** Custom: fan with a wrench. */
export const fanCustom = () =>
  svg(
    '0 0 24 24',
    `<g transform="translate(-1.6 -1.2) scale(.86)">${rotations(8, `<path d="M12 9.1C11.3 6.8 12.3 4.3 14.8 3.3"/><path d="M13.3 9.5C13.9 7.5 15.6 6.1 17.7 5.9" opacity=".75"/>`)}<circle cx="12" cy="12" r="2.5" fill="currentColor" stroke="none"/></g>` +
      `<path d="M22.2 13.6a3 3 0 0 1-3.9 3.8l-3.6 3.6a1.2 1.2 0 0 1-1.7-1.7l3.6-3.6a3 3 0 0 1 3.8-3.9l-1.8 1.8.4 1.4 1.4.4z" fill="var(--seg-bg, #111)" stroke-width="1.3"/>`,
    'stroke-width="1.3"',
  );

export const cpu = () =>
  svg(
    '0 0 24 24',
    `<rect x="5" y="5" width="14" height="14" rx="2"/><rect x="8.3" y="8.3" width="7.4" height="7.4" rx="1"/>` +
      [9, 12, 15].map((p) => `<path d="M${p} 2v3M${p} 19v3M2 ${p}h3M19 ${p}h3"/>`).join(''),
    'stroke-width="1.25"',
  );

export const gpu = () =>
  svg(
    '0 0 32 24',
    `<path d="M1.8 3.2h2.4V21"/><rect x="4.2" y="5" width="25.8" height="13.2" rx="2.2"/>` +
      `<path d="M13.4 8.6a3.3 3.3 0 1 0 0 5.8"/><circle cx="21.6" cy="11.6" r="4.1"/><circle cx="21.6" cy="11.6" r="1.4"/>` +
      `<path d="M8 18.2v2.6h14.4v-2.6M10.4 20.8v-1.3M13 20.8v-1.3M15.6 20.8v-1.3M18.2 20.8v-1.3"/>`,
    'stroke-width="1.25"',
  );

export const leaf = () =>
  svg(
    '0 0 24 24',
    `<path d="M5.5 19.2C4.2 11.5 8.8 5.2 19.6 4.2c.7 10.1-4.4 15.8-12.4 15.3z" fill="currentColor" fill-opacity=".85" stroke-width="1"/>` +
      `<path d="M5.2 20.2 14.6 9.4" stroke="var(--seg-bg, #111)" stroke-width="1.3"/>`,
  );

/** NitroSense "Default": inverted shield outline with a chevron (same geometry as the tray icon). */
export const nitro = () =>
  `<svg viewBox="0 0 32 32" aria-hidden="true"><path fill="currentColor" fill-rule="evenodd" d="M2.5 5h27L16 29zM8.2 8.3h15.6L16 22.2z"/><path fill="currentColor" d="M10.6 10.2h3.3l2.1 3.9 2.1-3.9h3.3L16 19.8z"/></svg>`;

export const performance = () =>
  svg(
    '0 0 24 24',
    `<path d="M20.2 13.2A8.3 8.3 0 1 1 15 4.4"/><path d="M15.2 2.2 15.6 4.9 12.9 5.6"/>` +
      `<path d="M12 12 16.4 7.9"/><circle cx="12" cy="12" r="2.1" fill="currentColor" stroke="none"/><path d="M7.4 14.8a5.1 5.1 0 0 1 .3-5.9" opacity=".7"/>`,
    'stroke-width="1.6"',
  );

export const speedometer = () =>
  svg(
    '0 0 28 24',
    `<path d="M3.2 18.5a10.8 10.8 0 1 1 21.6 0z"/><path d="M14 18.2 18.8 11.4"/><circle cx="14" cy="18.2" r="1.4" fill="currentColor" stroke="none"/>` +
      `<path d="M6.2 13.2 7.6 14M14 7.9v1.6M21.8 13.2l-1.4.8M9.6 9.3l.9 1.3M18.4 9.3l-.9 1.3"/>`,
    'stroke-width="1.5"',
  );

export const monitor = () =>
  svg(
    '0 0 28 24',
    `<rect x="2.2" y="2.4" width="23.6" height="15.4" rx="1.6"/><path d="M10.4 21.8h7.2M14 17.8v4"/>` +
      `<path d="M5.4 11.4h3.2l1.9-3.8 3 7 2.3-5.2 1.6 2h5.1"/>`,
    'stroke-width="1.5"',
  );

export const gamepad = () =>
  svg(
    '0 0 30 24',
    `<path d="M8.4 5.2h13.2c3.1 0 5 2.2 5.6 5.4l1 5.6c.5 2.7-1.2 4.6-3.3 4.6-1.6 0-2.6-1-3.5-2.4l-1-1.6h-11l-1 1.6c-.9 1.4-1.9 2.4-3.5 2.4-2.1 0-3.8-1.9-3.3-4.6l1-5.6c.6-3.2 2.5-5.4 5.8-5.4z"/>` +
      `<path d="M9.6 9.6v5M7.1 12.1h5"/><circle cx="20.6" cy="10.4" r="1.1" fill="currentColor" stroke="none"/><circle cx="23" cy="13" r="1.1" fill="currentColor" stroke="none"/>`,
    'stroke-width="1.5"',
  );

/** Settings gear: eight evenly spaced teeth with rounded tips on a 7.9 root circle, open hub; the stroke
 *  matches the close glyph beside it once both are at header size. */
export const gear = () =>
  svg(
    '0 0 24 24',
    `<path d="M9.96 4.37L10.27 2.66A10.1 10.1 0 0 1 13.73 2.66L14.04 4.37A7.9 7.9 0 0 1 15.95 5.16L17.38 4.17A10.1 10.1 0 0 1 19.83 6.62L18.84 8.05A7.9 7.9 0 0 1 19.63 9.96L21.34 10.27A10.1 10.1 0 0 1 21.34 13.73L19.63 14.04A7.9 7.9 0 0 1 18.84 15.95L19.83 17.38A10.1 10.1 0 0 1 17.38 19.83L15.95 18.84A7.9 7.9 0 0 1 14.04 19.63L13.73 21.34A10.1 10.1 0 0 1 10.27 21.34L9.96 19.63A7.9 7.9 0 0 1 8.05 18.84L6.62 19.83A10.1 10.1 0 0 1 4.17 17.38L5.16 15.95A7.9 7.9 0 0 1 4.37 14.04L2.66 13.73A10.1 10.1 0 0 1 2.66 10.27L4.37 9.96A7.9 7.9 0 0 1 5.16 8.05L4.17 6.62A10.1 10.1 0 0 1 6.62 4.17L8.05 5.16A7.9 7.9 0 0 1 9.96 4.37Z"/><circle cx="12" cy="12" r="3.5"/>`,
    'stroke-width="1.8"',
  );

export const close = () => svg('0 0 24 24', `<path d="M5 5 19 19M19 5 5 19"/>`, 'stroke-width="2.2"');
export const chevron = () => svg('0 0 24 24', `<path d="M9 5.5 15.5 12 9 18.5"/>`, 'stroke-width="2.2"');
export const back = () => svg('0 0 24 24', `<path d="M15 5.5 8.5 12l6.5 6.5"/>`, 'stroke-width="2.2"');
export const chart = () => svg('0 0 24 24', `<path d="M3 20h18M5 16l4-5 3 3 4-6 3 4"/>`, 'stroke-width="1.7"');
export const curve = () => svg('0 0 24 24', `<path d="M3 20h18M3 20V4"/><path d="M5 17c4 0 5-3 7-6s3-5 7-6"/><circle cx="12" cy="11" r="1.5" fill="currentColor"/>`, 'stroke-width="1.7"');

export const ICONS = {
  fan, fanMax, fanCustom, cpu, gpu, leaf, nitro, performance, speedometer, monitor, gamepad, gear, close, chevron, back, chart, curve,
};

/** Replaces every `[data-icon]` placeholder under `root` with its SVG (idempotent). */
export function hydrate(root) {
  for (const el of root.querySelectorAll('[data-icon]')) {
    const make = ICONS[el.dataset.icon];
    if (make && !el.querySelector('svg')) el.insertAdjacentHTML('afterbegin', make());
  }
}
