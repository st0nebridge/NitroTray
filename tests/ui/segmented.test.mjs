import { test } from 'node:test';
import assert from 'node:assert/strict';
import { click, errors, key, loadPage } from './helpers/dom.mjs';
import * as seg from '../../src/ui/web/js/segmented.js';

function setup() {
  const win = loadPage();
  const group = win.document.getElementById('fan-seg');
  const picks = [];
  seg.bind(group, (value, disabled) => picks.push(value ?? `disabled:${disabled}`));
  return { win, group, picks, btn: (v) => group.querySelector(`[data-value="${v}"]`) };
}

test('render reflects selection, pending, disabled and roving tabindex', () => {
  const { group, btn } = setup();
  seg.render(group, { value: 'max', pending: 'custom', enabled: { auto: true, max: true, custom: false }, reason: 'why' });
  assert.equal(btn('max').getAttribute('aria-checked'), 'true');
  assert.equal(btn('auto').getAttribute('aria-checked'), 'false');
  assert.equal(btn('custom').getAttribute('aria-disabled'), 'true');
  assert.equal(btn('custom').title, 'why');
  assert.equal(btn('custom').getAttribute('aria-busy'), 'true');
  assert.equal(btn('auto').hasAttribute('aria-busy'), false);
  assert.deepEqual(seg.buttons(group).map((b) => b.tabIndex), [-1, 0, -1]);
  seg.render(group, {});
  assert.deepEqual(seg.buttons(group).map((b) => b.tabIndex), [0, -1, -1], 'first enabled when nothing selected');
  seg.render(group, { enabled: { auto: false, max: true, custom: true } });
  assert.deepEqual(seg.buttons(group).map((b) => b.tabIndex), [-1, 0, -1], 'skips disabled');
  seg.render(group, { enabled: { auto: false, max: false, custom: false } });
  assert.equal(seg.buttons(group)[0].tabIndex, 0, 'something stays focusable');
});

test('clicks select enabled options and report disabled ones', () => {
  const { group, picks, btn } = setup();
  seg.render(group, { enabled: { auto: true, max: false, custom: true } });
  click(btn('auto'));
  click(btn('max'));
  click(btn('custom').querySelector('span'));
  click(group);
  assert.deepEqual(picks, ['auto', 'disabled:max', 'custom']);
});

test('arrow keys move focus (skipping disabled, wrapping); Enter/Space activate', () => {
  const { win, group, picks, btn } = setup();
  seg.render(group, { value: 'auto', enabled: { auto: true, max: false, custom: true } });
  btn('auto').focus();
  key(btn('auto'), 'ArrowRight');
  assert.equal(win.document.activeElement, btn('custom'), 'skips disabled Max');
  key(btn('custom'), 'ArrowDown');
  assert.equal(win.document.activeElement, btn('auto'), 'wraps');
  key(btn('auto'), 'ArrowLeft');
  assert.equal(win.document.activeElement, btn('custom'));
  key(btn('custom'), 'ArrowUp');
  assert.equal(win.document.activeElement, btn('auto'));
  key(btn('auto'), 'End');
  assert.equal(win.document.activeElement, btn('custom'));
  key(btn('custom'), 'Home');
  assert.equal(win.document.activeElement, btn('auto'));
  assert.equal(btn('auto').tabIndex, 0);
  key(btn('auto'), 'Enter');
  key(btn('auto'), ' ');
  key(btn('auto'), 'x');
  assert.deepEqual(picks, ['auto', 'auto']);
  key(group, 'ArrowRight');
});

test('focus stays put when every other option is disabled', () => {
  const { win, group, btn } = setup();
  seg.render(group, { value: 'auto', enabled: { auto: true, max: false, custom: false } });
  btn('auto').focus();
  key(btn('auto'), 'ArrowRight');
  assert.equal(win.document.activeElement, btn('auto'));
  assert.equal(seg.isEnabled(btn('max')), false);
});

test('render: exact ARIA values, busy cleared, titles only on disabled options', () => {
  const { group, btn } = setup();
  seg.render(group, { pending: 'custom', enabled: { auto: true, max: false, custom: true } });
  assert.equal(btn('auto').getAttribute('aria-disabled'), 'false');
  assert.equal(btn('auto').title, '');
  assert.equal(btn('max').title, '', 'no reason given, no title');
  seg.render(group, { pending: null });
  assert.equal(btn('custom').hasAttribute('aria-busy'), false, 'busy cleared when the change lands');
});

test('keyboard: direction, roving tabindex, preventDefault, ignored targets', () => {
  const { win, group, picks, btn } = setup();
  const errs = errors(win);
  seg.render(group, { value: 'auto' });
  btn('auto').focus();
  const right = key(btn('auto'), 'ArrowRight');
  assert.equal(win.document.activeElement, btn('max'), 'next, not previous');
  assert.equal(right.defaultPrevented, true);
  assert.deepEqual(seg.buttons(group).map((b) => b.tabIndex), [-1, 0, -1]);
  key(btn('max'), 'ArrowLeft');
  assert.equal(win.document.activeElement, btn('auto'), 'previous, not next');
  key(btn('auto'), 'x');
  assert.deepEqual(picks, [], 'other keys do nothing');
  assert.equal(key(btn('auto'), 'Enter').defaultPrevented, true);
  assert.deepEqual(picks, ['auto']);
  key(btn('auto'), ' ');
  assert.deepEqual(picks, ['auto', 'auto']);
  btn('auto').blur();
  key(group, 'ArrowRight');
  assert.equal(win.document.activeElement, win.document.body, 'keys on the group itself are ignored');
  click(group);
  seg.render(group, { enabled: { auto: false, max: false, custom: false } });
  btn('max').focus();
  key(btn('max'), 'ArrowRight');
  key(btn('max'), 'Home');
  assert.deepEqual(errs, [], 'all disabled: nothing to move to, nothing thrown');
});
