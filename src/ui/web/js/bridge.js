/**
 * @module bridge
 * @description Page ↔ host messaging: commands go to Rust via `window.ipc.postMessage`; Rust calls the
 *              functions registered on `window.nitro`.
 *
 * @input  Command objects `{cmd: ..., ...}`; handler maps for `window.nitro`.
 * @output Serialized JSON posted to the host, or a `nitro:command` DOM event when previewing in a browser.
 * @dependencies none
 */

/** Sends a command to the host. Outside WebView2 (preview/tests) it becomes a DOM event. */
export function send(command, win = globalThis) {
  const message = JSON.stringify(command);
  const ipc = win.ipc;
  if (ipc && typeof ipc.postMessage === 'function') {
    ipc.postMessage(message);
  } else if (typeof win.dispatchEvent === 'function') {
    win.dispatchEvent(new win.CustomEvent('nitro:command', { detail: JSON.parse(message) }));
  }
  return message;
}

/** Registers host-callable functions on `window.nitro` (merging with existing ones). */
export function expose(handlers, win = globalThis) {
  win.nitro = Object.assign(win.nitro || {}, handlers);
  return win.nitro;
}

/** Applies theme/accent/motion preferences to the document root. */
export function applyPrefs(prefs, doc = globalThis.document) {
  if (!prefs || !doc) return;
  const root = doc.documentElement;
  root.dataset.theme = prefs.theme === 'light' ? 'light' : 'dark';
  root.dataset.accent = ['subtle', 'normal', 'vivid'].includes(prefs.accent) ? prefs.accent : 'normal';
  root.dataset.reducedMotion = prefs.reduced_motion ? 'true' : 'false';
  root.dataset.transparency = prefs.transparency ? 'true' : 'false';
}
