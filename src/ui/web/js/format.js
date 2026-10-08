/**
 * @module format
 * @description Pure display formatting for telemetry values (placeholders, tabular numbers).
 *
 * @input  Nullable numbers from the Rust UI state.
 * @output Strings/classes for rendering; `--` for anything missing (never invented).
 * @dependencies none
 */

export const DASH = '--';

export function int(value) {
  return Number.isFinite(value) ? String(Math.round(value)) : DASH;
}

export function pct(value) {
  return Number.isFinite(value) ? `${Math.round(value)}%` : `${DASH}%`;
}

/** CSS class for a temperature level from Rust (`normal|warm|hot|critical|unknown`). */
export function levelClass(level) {
  const known = ['normal', 'warm', 'hot', 'critical'];
  return `lvl-${known.includes(level) ? level : 'unknown'}`;
}

/** Ring dash array for a 0–100 fill (pathLength = 100); no arc when unknown. */
export function ringDash(ringPct) {
  if (!Number.isFinite(ringPct) || ringPct <= 0) return '0 100';
  const v = Math.min(100, Math.max(0, ringPct));
  return `${v.toFixed(1)} ${(100 - v).toFixed(1)}`;
}

/** Fan cell text: RPM, "Unavailable" or "Off". */
export function fanValue(fan) {
  if (!fan || fan.state === 'unavailable') return { text: 'Unavailable', na: true };
  if (fan.state === 'disabled') return { text: 'Off', na: true };
  return { text: int(fan.rpm), na: !Number.isFinite(fan.rpm) };
}

/** Accessible description of a fan cell. */
export function fanLabel(name, fan) {
  const v = fanValue(fan);
  return v.na ? `${name} fan ${v.text.toLowerCase()}` : `${name} fan ${v.text} RPM`;
}
