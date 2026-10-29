//! @module unlock::ptx
//! @description The tensor probe's GPU code: a loop of independent `mma.sync` chains in PTX, once with FP32
//! and once with FP16 accumulate. The driver compiles it for the installed GPU, so no CUDA toolkit is needed.
//!
//! @input  Which accumulate type.
//! @output PTX source text (the caller appends the terminator), and the work count for a launch.
//! @dependencies none
use std::fmt::Write;

/// Independent accumulator chains per warp; enough in flight to keep the tensor pipes busy.
pub const CHAINS: usize = 8;
pub const THREADS: usize = 128;
pub const BLOCKS_PER_SM: usize = 8;
pub const ENTRY: &str = "probe";
/// Loop trips of a launch that is timed, and of the one that warms the GPU up.
pub const ITERS: u32 = 60_000;
pub const WARMUP_ITERS: u32 = 2_000;
/// Timed launches per accumulate type; the best counts, as a clock dip or a busy GPU only slows one.
pub const ROUNDS: usize = 3;
/// Work of one `mma.sync.m16n8k16` per warp: 2 * 16 * 8 * 16.
const FLOPS_PER_MMA: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accumulate {
    F32,
    F16,
}

/// Operations a launch of `blocks` blocks of `THREADS` threads, `iters` trips each, performs.
pub fn flops(blocks: usize, iters: usize) -> f64 {
    let warps = blocks * THREADS / 32;
    (warps * iters * CHAINS * FLOPS_PER_MMA) as f64
}

pub fn kernel(acc: Accumulate) -> String {
    let (regs, count, ty, zero, store, fold) = match acc {
        Accumulate::F32 => (4, "f32", "f32", "mov.f32 {r}, 0f00000000;", "st.global.f32", "add.f32"),
        Accumulate::F16 => (2, "b32", "f16", "mov.b32 {r}, 0;", "st.global.b32", "xor.b32"),
    };
    let names = |chain: usize| (0..regs).map(|j| format!("c{chain}_{j}")).collect::<Vec<_>>();
    let mut declare = String::new();
    let mut init = String::new();
    let mut body = String::new();
    let mut total = String::new();
    for chain in 0..CHAINS {
        let list = names(chain).join(",");
        writeln!(declare, "  .reg .{count} c{chain}_<{regs}>;").unwrap();
        for r in names(chain) {
            writeln!(init, "  {}", zero.replace("{r}", &r)).unwrap();
        }
        let shape = match acc {
            Accumulate::F32 => "f32.f16.f16.f32",
            Accumulate::F16 => "f16.f16.f16.f16",
        };
        writeln!(
            body,
            "  mma.sync.aligned.m16n8k16.row.col.{shape} {{{list}}}, {{a0,a1,a2,a3}}, {{b0,b1}}, {{{list}}};"
        )
        .unwrap();
        for r in names(chain) {
            writeln!(total, "  {fold} t, t, {r};").unwrap();
        }
    }
    let t_decl = if ty == "f32" { ".reg .f32 t;" } else { ".reg .b32 t;" };
    let t_zero = if ty == "f32" { "mov.f32 t, 0f00000000;" } else { "mov.b32 t, 0;" };
    format!(
        r#".version 7.0
.target sm_80
.address_size 64
.visible .entry {ENTRY}(.param .u64 out, .param .u32 iters)
{{
  .reg .b32 a<4>;
  .reg .b32 b<2>;
  .reg .u32 n, tid;
  .reg .pred p;
  .reg .u64 o, off;
  {t_decl}
{declare}  ld.param.u64 o, [out];
  ld.param.u32 n, [iters];
  mov.b32 a0, 0x2C002C00;
  mov.b32 a1, 0x2C402C40;
  mov.b32 a2, 0x2C802C80;
  mov.b32 a3, 0x2CC02CC0;
  mov.b32 b0, 0x2C002C00;
  mov.b32 b1, 0x2C402C40;
{init}  {t_zero}
LOOP:
{body}  sub.u32 n, n, 1;
  setp.ne.u32 p, n, 0;
  @p bra LOOP;
{total}  mov.u32 tid, %tid.x;
  cvt.u64.u32 off, tid;
  shl.b64 off, off, 2;
  cvta.to.global.u64 o, o;
  add.u64 o, o, off;
  {store} [o], t;
  ret;
}}
"#
    )
}
