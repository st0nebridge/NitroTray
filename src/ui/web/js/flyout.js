/**
 * @module flyout
 * @description Hover flyout entry point: exposes `window.nitro.onFlyout` and announces readiness.
 *
 * @input  `FlyoutState` pushed by the host via `window.nitro.onFlyout(state)`.
 * @output A `ready` command; nothing else (the flyout is display-only and never takes focus).
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

document.addEventListener('contextmenu', (e) => e.preventDefault());
send({ cmd: 'ready' });
