# Coverage waiver: GPU and EFI-partition glue

- **Scope:** `src/unlock/cuda.rs` (the CUDA driver binding that runs the tensor probe) and `src/service/esp.rs`
  (finding the unlocker's log directory among the volumes).
- **Why:** they only bind OS and driver entry points. The first needs an NVIDIA GPU of compute capability 8.0 or
  newer and wakes it; the second needs an elevated session (the helper's LocalSystem account) and a machine the
  UEFI unlocker has run on. Neither can run in an ordinary unit-test process on every machine.
- **What they call is tested:** every decision lives in tested modules: `unlock::{ptx, state, boot_log, tracker,
  view}`, `service::unlock_log` (directory scan, size limit, newest-run rule), `app::tensor` (worker) and the IPC
  command that carries the report. Mutation testing of those modules: no surviving mutant, one equivalent
  mutant excluded with its reason in `.cargo/mutants.toml`.
- **Compensating evidence:** `tests/unit/unlock_cuda_live_test.rs` and `tests/unit/esp_live_test.rs` are
  `#[ignore]` live tests, run on the AN515-58 (RTX 4060 Laptop): the probe read ratio 0.50 with the throttle on;
  the partition was read by volume path, elevated and without mounting it, returning the newest boot report; an
  end-to-end run (elevated helper, isolated app, popup and details popup read through DevTools) showed the
  expected "Unlock lost" for a boot that unlocked and was then reset.
- **Gate:** the coverage gate is evaluated on the logic scope, which excludes these two files together with the
  files of `ui-glue.md`; the whole-crate figure is always reported alongside it.
- **Expiry / review:** revisit when the unlocked reading (ratio about 1.0) has been observed on this machine, and
  if a GPU-less fake of the driver binding becomes worth maintaining.
