import { test } from 'node:test';
import assert from 'node:assert/strict';
import { loadPage } from './helpers/dom.mjs';
import { applyPrefs, expose, send } from '../../src/ui/web/js/bridge.js';

test('send posts JSON to the WebView2 host when present', () => {
  const posted = [];
  const win = { ipc: { postMessage: (m) => posted.push(m) } };
  const msg = send({ cmd: 'close_popup' }, win);
  assert.equal(msg, '{"cmd":"close_popup"}');
  assert.deepEqual(posted, ['{"cmd":"close_popup"}']);
});

test('send falls back to a DOM event outside the host', () => {
  const win = loadPage();
  let got = null;
  win.addEventListener('nitro:command', (e) => { got = e.detail; });
  send({ cmd: 'set_fan_mode', mode: 'max' }, win);
  assert.deepEqual(got, { cmd: 'set_fan_mode', mode: 'max' });
  assert.equal(send({ cmd: 'ready' }, {}), '{"cmd":"ready"}', 'no host, no dispatcher: still serialises');
});

test('expose merges handlers onto window.nitro', () => {
  const win = {};
  expose({ a: 1 }, win);
  const api = expose({ b: 2 }, win);
  assert.deepEqual({ ...api }, { a: 1, b: 2 });
});

test('applyPrefs sets root data attributes and sanitises unknown values', () => {
  const win = loadPage();
  const doc = win.document;
  applyPrefs({ theme: 'light', accent: 'vivid', reduced_motion: true, transparency: true }, doc);
  const d = doc.documentElement.dataset;
  assert.deepEqual([d.theme, d.accent, d.reducedMotion, d.transparency], ['light', 'vivid', 'true', 'true']);
  applyPrefs({ theme: 'neon', accent: 'loud' }, doc);
  assert.deepEqual([d.theme, d.accent, d.reducedMotion, d.transparency], ['dark', 'normal', 'false', 'false']);
  applyPrefs(null, doc);
  applyPrefs({ theme: 'dark' }, null);
});

test('send ignores an ipc object without postMessage, and keeps known accents', () => {
  const win = loadPage();
  win.ipc = { postMessage: 'not a function' };
  let got = null;
  win.addEventListener('nitro:command', (e) => { got = e.detail; });
  send({ cmd: 'ready' }, win);
  assert.deepEqual(got, { cmd: 'ready' });
  for (const accent of ['subtle', 'normal', 'vivid']) {
    applyPrefs({ accent }, win.document);
    assert.equal(win.document.documentElement.dataset.accent, accent);
  }
});
