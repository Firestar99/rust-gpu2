//! Compilation backend pipeline: optimized IR to VCode / binemit.

use crate::dominator_tree::DominatorTree;
use crate::ir::Function;
use crate::isa::TargetIsa;
use crate::machinst::*;
use crate::settings::RegallocAlgorithm;
use crate::timing;
use crate::trace;

use regalloc2::{Algorithm, Allocation, Function as _, Inst, RegallocOptions};

/// Compile the given function down to VCode *without* allocating registers.
pub fn compile_vcode<B: LowerBackend + TargetIsa>(
    f: &Function,
    domtree: &DominatorTree,
    b: &B,
    abi: Callee<<<B as LowerBackend>::MInst as MachInst>::ABIMachineSpec>,
    emit_info: <B::MInst as MachInstEmit>::Info,
    sigs: SigSet,
    ctrl_plane: &mut ControlPlane,
) -> CodegenResult<VCode<B::MInst>> {
    // Compute lowered block order.
    let block_order = BlockLoweringOrder::new(f, domtree, ctrl_plane);

    // Build the lowering context.
    let lower =
        crate::machinst::Lower::new(f, abi, emit_info, block_order, sigs, b.flags().clone())?;

    // Lower the IR.
    let vcode = {
        log::debug!(
            "Number of CLIF instructions to lower: {}",
            f.dfg.num_insts()
        );
        log::debug!("Number of CLIF blocks to lower: {}", f.dfg.num_blocks());

        let _tt = timing::vcode_lower();
        lower.lower(b, ctrl_plane)?
    };

    log::debug!(
        "Number of lowered vcode instructions: {}",
        vcode.num_insts()
    );
    log::debug!("Number of lowered vcode blocks: {}", vcode.num_blocks());
    trace!("vcode from lowering: \n{:?}", vcode);
    Ok(vcode)
}

/// Compile the given function down to VCode with allocated registers, ready
/// for binary emission.
pub fn compile<B: LowerBackend + TargetIsa>(
    f: &Function,
    domtree: &DominatorTree,
    regalloc_ctx: &mut regalloc2::Ctx,
    b: &B,
    abi: Callee<<<B as LowerBackend>::MInst as MachInst>::ABIMachineSpec>,
    emit_info: <B::MInst as MachInstEmit>::Info,
    sigs: SigSet,
    ctrl_plane: &mut ControlPlane,
) -> CodegenResult<VCode<B::MInst>> {
    let vcode = compile_vcode(f, domtree, b, abi, emit_info, sigs, ctrl_plane)?;

    // Perform register allocation.
    {
        let _tt = timing::regalloc();
        let mut options = RegallocOptions::default();
        options.verbose_log = b.flags().regalloc_verbose_logs();

        if cfg!(debug_assertions) {
            options.validate_ssa = true;
        }

        options.algorithm = match b.flags().regalloc_algorithm() {
            RegallocAlgorithm::Backtracking => Algorithm::Ion,
            RegallocAlgorithm::SinglePass => Algorithm::Fastalloc,
        };

        regalloc2::run_with_ctx(&vcode, vcode.abi.machine_env(), &options, regalloc_ctx)
            .map_err(|err| {
                log::error!(
                    "Register allocation error for vcode\n\
                     {vcode:?}\n\
                     Error: {err:?}\n\
                     CLIF for error:\n\
                     {f:?}",
                );
                err
            })
            .expect("register allocation");
    }

    // Run the regalloc checker, if requested.
    if b.flags().regalloc_checker() {
        let _tt = timing::regalloc_checker();
        let mut checker = regalloc2::checker::Checker::new(&vcode, &vcode.abi.machine_env());
        checker.prepare(&regalloc_ctx.output);
        checker
            .run()
            .map_err(|err| {
                log::error!("Register allocation checker errors:\n{err:?}\nfor vcode:\n{vcode:?}");
                err
            })
            .expect("register allocation checker");
    }

    Ok(vcode)
}

/// Compile the given function down to VCode with the [`fake_regalloc`] to keep the virtual registers.
pub fn compile_ssa<B: LowerBackend + TargetIsa>(
    f: &Function,
    domtree: &DominatorTree,
    regalloc_ctx: &mut regalloc2::Ctx,
    b: &B,
    abi: Callee<<<B as LowerBackend>::MInst as MachInst>::ABIMachineSpec>,
    emit_info: <B::MInst as MachInstEmit>::Info,
    sigs: SigSet,
    ctrl_plane: &mut ControlPlane,
) -> CodegenResult<VCode<B::MInst>> {
    let vcode = compile_vcode(f, domtree, b, abi, emit_info, sigs, ctrl_plane)?;
    fake_regalloc(&vcode, &mut regalloc_ctx.output);
    Ok(vcode)
}

/// A fake regalloc that keeps all registers as virtual registers.
pub fn fake_regalloc<I: VCodeInst>(func: &VCode<I>, output: &mut regalloc2::Output) {
    output.num_spillslots = 0;
    output.edits.clear();
    output.allocs.clear();
    output.inst_alloc_offsets.clear();
    output.debug_locations.clear();
    output.stats = Default::default();

    // adopted from `regalloc2::fastalloc::Allocs::new()`
    output.inst_alloc_offsets.reserve_exact(func.num_insts());
    let mut no_of_operands = 0;
    for inst in 0..func.num_insts() {
        let operands_len = func.inst_operands(Inst::new(inst)).len();
        output.inst_alloc_offsets.push(no_of_operands as u32);
        no_of_operands += operands_len;
    }
    output.allocs.resize(no_of_operands, Allocation::none());
}
