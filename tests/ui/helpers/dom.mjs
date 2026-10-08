/**
 * @module dom
 * @description Test helper: a jsdom window with a real page's markup (scripts stripped) and browser globals.
 */
import { readFileSync } from 'node:fs';
import { JSDOM } from 'jsdom';

const WEB = new URL('../../../src/ui/web/', import.meta.url);

/** Loads `page` markup into a fresh jsdom and installs window/document globals. */
export function loadPage(page = 'popup.html', url = 'http://nitro.localhost/') {
  const html = readFileSync(new URL(page, WEB), 'utf8').replace(/<script[\s\S]*?<\/script>/g, '');
  const dom = new JSDOM(html, { pretendToBeVisual: true, url });
  const { window } = dom;
  Object.assign(globalThis, {
    window,
    document: window.document,
    CustomEvent: window.CustomEvent,
    location: window.location,
    requestAnimationFrame: (cb) => setTimeout(cb, 0),
  });
  return window;
}

/** Records every command a view sends. */
export function spy() {
  const calls = [];
  const send = (cmd) => { calls.push(cmd); return JSON.stringify(cmd); };
  return { calls, send };
}

/** Dispatches a cancelable keydown and returns the event (check `defaultPrevented`). */
export function key(el, k) {
  const ev = new el.ownerDocument.defaultView.KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true });
  el.dispatchEvent(ev);
  return ev;
}

/** Collects exceptions thrown by event listeners (jsdom reports them as window `error` events). */
export function errors(win) {
  const seen = [];
  win.addEventListener('error', (e) => { seen.push(e.error?.message || e.message); e.preventDefault(); });
  return seen;
}

export function click(el) {
  el.dispatchEvent(new el.ownerDocument.defaultView.MouseEvent('click', { bubbles: true }));
}

export const readJson = (rel) => JSON.parse(readFileSync(new URL(rel, import.meta.url), 'utf8'));
