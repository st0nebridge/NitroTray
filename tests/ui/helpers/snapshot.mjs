/**
 * @module snapshot
 * @description Test helper: golden-file snapshots for rendered markup and canvas draw logs.
 *              `NITROTRAY_WRITE_FIXTURES=1` rewrites the goldens (same switch as the Rust fixture writer);
 *              otherwise a missing or different golden fails the test.
 */
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import assert from 'node:assert/strict';

const DIR = new URL('../fixtures/snapshots/', import.meta.url);

/** Compares `text` with `fixtures/snapshots/<name>.txt`. */
export function matchSnapshot(name, text) {
  const file = new URL(`${name}.txt`, DIR);
  if (process.env.NITROTRAY_WRITE_FIXTURES === '1') {
    mkdirSync(DIR, { recursive: true });
    writeFileSync(file, text, 'utf8');
    return;
  }
  assert.ok(existsSync(file), `missing golden ${name}.txt: run with NITROTRAY_WRITE_FIXTURES=1 and review it`);
  assert.equal(text, readFileSync(file, 'utf8').replace(/\r\n/g, '\n'), `${name} differs from its golden`);
}

/** Breaks markup before each tag so goldens diff line by line. */
export const lines = (html) => html.replace(/></g, '>\n<');

const fmt = (v) => (typeof v === 'number' ? String(Math.round(v * 1000) / 1000) : JSON.stringify(v));

/** A 2D context that records every call and property write, in order, as text lines. */
export function recordingContext() {
  const log = [];
  const ctx = new Proxy({}, {
    get: (target, p) => (p in target ? target[p] : (...args) => { log.push(`${String(p)}(${args.map(fmt).join(', ')})`); }),
    set: (target, p, v) => { log.push(`${String(p)} = ${fmt(v)}`); target[p] = v; return true; },
  });
  return { ctx, log };
}
