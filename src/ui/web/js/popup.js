/**
 * @module popup
 * @description Popup entry point: builds the view, exposes `window.nitro.onState`, handles Escape.
 *
 * @input  State pushed by the host via `window.nitro.onState(state)`.
 * @output Commands to the host; Escape → close_popup.
 * @dependencies bridge, popup-view
 */
import { applyPrefs, expose, send } from './bridge.js';
import { createPopupView } from './popup-view.js';

const view = createPopupView(document.getElementById('popup'), send);

expose({
  onState(state) {
    applyPrefs(state.prefs);
    view.render(state);
  },
  focusFirst() {
    const target = document.querySelector('#fan-seg [tabindex="0"]') || document.getElementById('btn-close');
    target?.focus({ preventScroll: true });
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
