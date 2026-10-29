//! @module unlock_ptx_test
//! @description The tensor probe's PTX: the two kernels differ only in the accumulator type, keep every chain
//! independent, and the work count used for the rate matches what the kernel issues.
use nitrotray::unlock::ptx::{flops, kernel, Accumulate, CHAINS, ENTRY, THREADS};

fn loop_body(ptx: &str) -> &str {
    let start = ptx.find("LOOP:").expect("loop label") + "LOOP:".len();
    let end = ptx.find("@p bra LOOP;").expect("loop branch");
    &ptx[start..end]
}

#[test]
fn each_kernel_issues_one_mma_per_chain_per_iteration() {
    for acc in [Accumulate::F32, Accumulate::F16] {
        let ptx = kernel(acc);
        let mma = loop_body(&ptx).lines().filter(|l| l.trim_start().starts_with("mma.sync")).count();
        assert_eq!(mma, CHAINS, "{acc:?}");
    }
}

#[test]
fn the_kernels_differ_in_the_accumulator_only() {
    let f32k = kernel(Accumulate::F32);
    let f16k = kernel(Accumulate::F16);
    assert!(f32k.contains("m16n8k16.row.col.f32.f16.f16.f32"));
    assert!(f16k.contains("m16n8k16.row.col.f16.f16.f16.f16"));
    assert!(!f32k.contains(".f16.f16.f16.f16"));
    assert!(!f16k.contains(".f32.f16.f16.f32"));
}

#[test]
fn every_chain_accumulates_into_its_own_registers() {
    let ptx = kernel(Accumulate::F32);
    let body = loop_body(&ptx);
    for chain in 0..CHAINS {
        let regs = format!("{{c{chain}_0,c{chain}_1,c{chain}_2,c{chain}_3}}");
        assert_eq!(body.matches(&regs).count(), 2, "chain {chain} reads and writes its own accumulator");
    }
}

#[test]
fn the_result_is_stored_so_the_work_cannot_be_dropped() {
    assert!(kernel(Accumulate::F32).contains("st.global.f32"));
    assert!(kernel(Accumulate::F16).contains("st.global.b32"));
}

#[test]
fn both_kernels_share_the_entry_point_and_target() {
    for acc in [Accumulate::F32, Accumulate::F16] {
        let ptx = kernel(acc);
        assert!(ptx.contains(&format!(".visible .entry {ENTRY}(")), "{acc:?}");
        assert!(ptx.contains(".target sm_80"), "mma m16n8k16 needs sm_80, and the driver JITs it up");
        assert!(ptx.contains(".address_size 64"));
        assert!(!ptx.contains('\0'), "the caller appends the terminator");
    }
}

#[test]
fn the_work_count_is_four_thousand_ninety_six_per_instruction() {
    // one m16n8k16 multiply-accumulate is 2 * 16 * 8 * 16 operations per warp
    let blocks = 24 * 8;
    let warps = blocks * THREADS / 32;
    assert_eq!(flops(blocks, 1000), (warps * 1000 * CHAINS * 4096) as f64);
    assert_eq!(flops(0, 1000), 0.0);
}

#[test]
fn the_summing_register_has_the_type_of_the_accumulators() {
    let f32k = kernel(Accumulate::F32);
    assert!(
        f32k.contains(".reg .f32 t;")
            && f32k.contains("mov.f32 t, 0f00000000;")
            && f32k.contains("add.f32 t, t, c0_0;")
    );
    assert!(!f32k.contains(".reg .b32 t;") && !f32k.contains("xor.b32"));
    let f16k = kernel(Accumulate::F16);
    assert!(f16k.contains(".reg .b32 t;") && f16k.contains("mov.b32 t, 0;") && f16k.contains("xor.b32 t, t, c0_0;"));
    assert!(!f16k.contains(".reg .f32 t;") && !f16k.contains("add.f32"));
}
