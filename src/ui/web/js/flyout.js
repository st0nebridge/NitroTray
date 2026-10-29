/**
 * @module flyout
 * @description Details popup entry point (opened by a click on a CPU or GPU tray icon): exposes
 * `window.nitro.onFlyout`, announces readiness and closes on Escape.
 *
 * @input  `FlyoutState` pushed by the host via `window.nitro.onFlyout(state)`.
 * @output `ready` and, on Escape, `close_popup`; nothing else (the popup is display-only).
 * @dependencies bridge, flyout-view
 */
import { applyPrefs, expose, send } from './bridge.js';
import { createFlyoutView } from './flyout-view.js';

const view = createFlyoutView(document.getElementById('flyout'));

expose({
  onFlyout(state) {
    applyPrefs(state?.prefs);
    view.render(state);
  },
});

document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape') {
    e.preventDefault();
    send({ cmd: 'close_popup' });
  }
});
document.addEventListener('contextmenu', (e) => e.preventDefault());
send({ cmd: 'ready' });
