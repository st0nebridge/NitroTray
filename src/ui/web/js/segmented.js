/**
 * @module segmented
 * @description Accessible segmented control: radiogroup semantics, roving tabindex,
 *              arrow/Home/End keys, Enter/Space activation, colour-independent selected state.
 *
 * @input  A `[role=radiogroup]` element containing `button[role=radio][data-value]`.
 * @output Calls `onSelect(value)` for enabled choices; `render()` reflects selection/pending/disabled.
 * @dependencies none
 */

const KEYS_NEXT = ['ArrowRight', 'ArrowDown'];
const KEYS_PREV = ['ArrowLeft', 'ArrowUp'];

export function buttons(group) {
  return Array.from(group.querySelectorAll('button[role="radio"]'));
}

export function isEnabled(button) {
  return button.getAttribute('aria-disabled') !== 'true';
}

/** Reflects state: `{ value, pending, enabled: {value: bool}, reason }`. */
export function render(group, { value = null, pending = null, enabled = {}, reason = '' } = {}) {
  const all = buttons(group);
  let focusable = null;
  for (const b of all) {
    const v = b.dataset.value;
    const checked = v === value;
    const on = enabled[v] !== false;
    b.setAttribute('aria-checked', checked ? 'true' : 'false');
    b.setAttribute('aria-disabled', on ? 'false' : 'true');
    if (v === pending) b.setAttribute('aria-busy', 'true');
    else b.removeAttribute('aria-busy');
    b.title = on ? '' : reason;
    if (checked) focusable = b;
  }
  const active = group.ownerDocument.activeElement;
  focusable = all.includes(active) ? active : focusable || all.find(isEnabled) || all[0];
  for (const b of all) b.tabIndex = b === focusable ? 0 : -1;
}

function move(group, from, step) {
  const all = buttons(group);
  const n = all.length;
  let i = all.indexOf(from);
  for (let k = 0; k < n; k += 1) {
    i = (i + step + n) % n;
    if (isEnabled(all[i])) return all[i];
  }
  return from;
}

/** Wires pointer and keyboard behaviour once. */
export function bind(group, onSelect) {
  const activate = (b) => {
    if (b && isEnabled(b)) onSelect(b.dataset.value);
    else if (b) onSelect(null, b.dataset.value);
  };
  group.addEventListener('click', (e) => activate(e.target.closest('button[role="radio"]')));
  group.addEventListener('keydown', (e) => {
    const current = e.target.closest('button[role="radio"]');
    if (!current) return;
    let target = null;
    if (KEYS_NEXT.includes(e.key)) target = move(group, current, 1);
    else if (KEYS_PREV.includes(e.key)) target = move(group, current, -1);
    else if (e.key === 'Home') target = buttons(group).find(isEnabled);
    else if (e.key === 'End') target = buttons(group).reverse().find(isEnabled);
    else if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      activate(current);
      return;
    } else return;
    e.preventDefault();
    for (const b of buttons(group)) b.tabIndex = b === target ? 0 : -1;
    target?.focus();
  });
}
