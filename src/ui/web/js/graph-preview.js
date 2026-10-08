/**
 * @module graph-preview
 * @description Live preview of a tray graph icon in the settings page, mirroring tray::graph_icon
 *              (background, translucent temperature fill, crisp lines, newest sample at the right).
 *
 * @input  A canvas, a GraphIconSettings-shaped object, oldest-first temperature and RPM samples.
 * @output Pixels on the canvas at the requested icon size.
 * @dependencies none
 */

export const FILL_ALPHA = 110 / 255;
export const MIN_AUTO_FAN_RPM = 3000;
export const AUTO_HEADROOM = 1.1;

export function fanScale(configured, peak) {
  return configured > 0 ? configured : Math.max((Number.isFinite(peak) ? peak : 0) * AUTO_HEADROOM, MIN_AUTO_FAN_RPM);
}

export function yFor(v, lo, hi, size) {
  const t = Math.min(1, Math.max(0, (v - lo) / Math.max(hi - lo, 1e-6)));
  return size - 0.5 - t * (size - 1);
}

export function renderPreview(canvas, s, temps, fans, size = 32) {
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext?.('2d');
  if (!ctx) return false;
  ctx.fillStyle = s.background_color;
  ctx.fillRect(0, 0, size, size);
  const series = (values, lo, hi, color, fill) => {
    const tail = values.slice(-size);
    const x0 = size - tail.length;
    const pts = tail.map((v, i) => (Number.isFinite(v) ? [x0 + i + 0.5, yFor(v, lo, hi, size)] : null));
    if (fill) {
      ctx.globalAlpha = FILL_ALPHA;
      ctx.fillStyle = color;
      for (const p of pts) if (p) ctx.fillRect(Math.floor(p[0]), p[1], 1, size - p[1]);
      ctx.globalAlpha = 1;
    }
    ctx.strokeStyle = color;
    ctx.lineWidth = Math.max(1, size / 16);
    ctx.beginPath();
    let pen = false;
    for (const p of pts) {
      if (!p) { pen = false; continue; }
      if (pen) ctx.lineTo(...p); else ctx.moveTo(...p);
      pen = true;
    }
    ctx.stroke();
  };
  if (s.show_temperature) series(temps, s.temperature_min_c, s.temperature_max_c, s.temperature_color, s.fill);
  if (s.show_fan) {
    const peak = Math.max(0, ...fans.filter(Number.isFinite));
    series(fans, 0, fanScale(s.fan_max_rpm, peak), s.fan_color, false);
  }
  return true;
}
