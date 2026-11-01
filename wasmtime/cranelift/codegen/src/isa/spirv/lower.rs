use crate::isa::spirv::SpirvBackend;
use crate::machinst::{InstOutput, Lower, LowerBackend};
use crate::{MachLabel, Reg};
use crate::isa::spirv::inst::Inst;

pub mod isle;

//=============================================================================
// Lowering-backend trait implementation.

impl LowerBackend for SpirvBackend {
    type MInst = Inst;

    fn lower(
        &self,
        ctx: &mut Lower<Inst>,
        ir_inst: crate::ir::entities::Inst,
    ) -> Option<InstOutput> {
        isle::lower(ctx, self, ir_inst)
    }

    fn lower_branch(
        &self,
        ctx: &mut Lower<Inst>,
        ir_inst: crate::ir::entities::Inst,
        targets: &[MachLabel],
    ) -> Option<()> {
        isle::lower_branch(ctx, self, ir_inst, targets)
    }

    fn maybe_pinned_reg(&self) -> Option<Reg> {
        // pinned register is a register that you want put anything in it.
        // right now riscv64 not support this feature.
        None
    }
}
