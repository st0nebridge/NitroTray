import { test } from 'node:test';
import assert from 'node:assert/strict';
import { loadPage } from './helpers/dom.mjs';
import { ICONS, hydrate } from '../../src/ui/web/js/icons.js';

test('every icon is a self-contained, id-free, colour-inheriting SVG', () => {
  for (const [name, make] of Object.entries(ICONS)) {
    const markup = make();
    assert.match(markup, /^<svg viewBox="0 0 \d+ \d+"/, name);
    assert.match(markup, /aria-hidden="true"/, name);
    assert.doesNotMatch(markup, /\sid="/, `${name} must not use ids (icons repeat on the page)`);
    assert.match(markup, /currentColor/, name);
  }
  assert.match(ICONS.fan(2), /stroke-width="2"/);
});

test('hydrate fills placeholders once', () => {
  const win = loadPage();
  const doc = win.document;
  hydrate(doc);
  const placeholders = [...doc.querySelectorAll('[data-icon]')];
  assert.ok(placeholders.length > 20);
  for (const el of placeholders) assert.equal(el.querySelectorAll('svg').length, 1, el.dataset.icon);
  hydrate(doc);
  for (const el of placeholders) assert.equal(el.querySelectorAll('svg').length, 1, 'idempotent');
  const unknown = doc.createElement('span');
  unknown.dataset.icon = 'nope';
  doc.body.append(unknown);
  hydrate(doc);
  assert.equal(unknown.innerHTML, '');
});
