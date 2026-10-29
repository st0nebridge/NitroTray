/**
 * @module unlock-views
 * @description Sample tensor-core lock views for the preview pages (`?unlock=` picks one), shaped like what
 *              `src/unlock/view.rs` sends: the five states, with and without the unlocker's boot line.
 */
const RUN = 'Unlocker run RUN0028: unlocked at boot';

export const UNLOCK_VIEWS = {
  none: null,
  unlocked: {
    state: 'unlocked', badge: 'Unlocked',
    detail: 'FP32-accumulate tensor rate is at full rate (ratio 1.00), checked 12 s ago.',
    summary: 'ratio 1.00, checked 12 s ago', ratio: 1.0, age_s: 12, boot: RUN,
  },
  locked: {
    state: 'locked', badge: 'Locked',
    detail: 'FP32-accumulate tensor rate is at half rate (ratio 0.50), checked 12 s ago.',
    summary: 'ratio 0.50, checked 12 s ago', ratio: 0.5, age_s: 12, boot: null,
  },
  lost: {
    state: 'lost', badge: 'Unlock lost',
    detail: 'Unlocked at boot (RUN0028) but at half rate now (ratio 0.50), checked 12 s ago: the RTX was reset after the unlock.',
    summary: 'ratio 0.50, checked 12 s ago', ratio: 0.5, age_s: 12, boot: RUN,
  },
  checking: {
    state: 'checking', badge: 'Checking', detail: 'Checking the tensor-core rate…',
    summary: '', ratio: null, age_s: null, boot: RUN,
  },
  unknown: {
    state: 'unknown', badge: '', detail: 'The tensor-core rate could not be checked: nvcuda.dll not found (no NVIDIA driver).',
    summary: '', ratio: null, age_s: null, boot: null,
  },
};
