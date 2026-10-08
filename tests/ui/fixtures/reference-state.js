/**
 * @module reference-state
 * @description UiState fixture reproducing the values shown in docs/reference/nitrosense-compact-reference.webp.
 * Note: the reference colours GPU 82 °C orange ("warm"); with the default GPU bands (80–89 red)
 * the live classifier would say "hot". Thresholds are user-configurable.
 */
export const referenceState = {
  cpu: { temp: 85, level: 'hot', util: 21, mhz: 3947 },
  gpu: { temp: 82, level: 'warm', util: 93, mhz: 2565 },
  cpu_fan: { state: 'ok', rpm: 7317, ring_pct: 60 },
  gpu_fan: { state: 'ok', rpm: 7692, ring_pct: 61 },
  fan_mode: 'auto',
  performance_mode: 'default',
  performance_other: null,
  capabilities: {
    cpu_fan: true, gpu_fan: true, auto_fan_mode: true, max_fan_mode: true, custom_fan_mode: true,
    quiet_mode: true, default_mode: true, performance_mode: true, cpu_temperature: true, gpu_temperature: true,
  },
  connection: 'connected',
  connection_text: 'Connected to the Acer firmware interface.',
  provider: 'acer-wmi',
  model: 'Nitro AN515-58',
  has_custom_profile: false,
  pending: null,
  notice: null,
  prefs: { theme: 'dark', accent: 'normal', reduced_motion: true, transparency: false, unit_symbol: '°C', unit: 'celsius' },
};

export const helperMissingState = {
  ...referenceState,
  cpu: { temp: null, level: 'unknown', util: 18, mhz: 4021 },
  gpu: { temp: 49, level: 'normal', util: 3, mhz: 210 },
  cpu_fan: { state: 'unavailable', rpm: null, ring_pct: null },
  gpu_fan: { state: 'unavailable', rpm: null, ring_pct: null },
  fan_mode: null,
  performance_mode: null,
  capabilities: Object.fromEntries(Object.keys(referenceState.capabilities).map((k) => [k, false])),
  connection: 'helper_unavailable',
  connection_text: 'The NitroTray helper service is not running. Install it from Settings › Integration.',
  notice: { kind: 'error', title: 'Could not switch to Performance mode.', detail: 'NitroSense service did not accept the request.' },
};
