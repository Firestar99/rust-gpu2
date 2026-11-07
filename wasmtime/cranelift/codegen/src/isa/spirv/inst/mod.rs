//! spirv instructions

pub use crate::isa::spirv::inst::emit::EmitInfo;
use crate::isa::spirv::lower::isle::generated_code::MInst;
pub use crate::isa::spirv::lower::isle::generated_code::{Def, MInst as Inst, SpvLabel, Use};
use crate::isa::ssa::{SsaAbi, SsaInst};
use crate::machinst::{
    ArgPair, CallType, Callee, MachTerminator, OperandVisitor, OperandVisitorImpl, RetPair,
    SmallInstVec,
};
use alloc::vec::Vec;
use regalloc2::{OperandConstraint, OperandKind, OperandPos};

mod emit;
mod reg;
mod ty;

pub use ty::*;

/// [`Callee`] for spirv
pub type SpirvCallee = Callee<SsaAbi<Inst>>;
/// A Vec of [`Use`]
pub type VecUse = Vec<Use>;

impl SsaInst for Inst {
    type F = super::spirv_settings::Flags;

    fn word_bits() -> u32 {
        64
    }

    fn gen_args(args: Vec<ArgPair>) -> Self {
        Inst::Args { args }
    }

    fn gen_rets(rets: Vec<RetPair>) -> Self {
        Inst::Rets { rets }
    }

    fn gen_return(_isa_flags: &Self::F) -> SmallInstVec<Self> {
        SmallInstVec::from_iter([Inst::Return])
    }

    fn gen_nop(_preferred_size: usize) -> Self {
        Inst::Nop
    }

    /// only used in wasmtime
    fn gen_nop_units() -> Vec<Vec<u8>> {
        Vec::new()
    }

    fn get_operands(&mut self, collector: &mut impl OperandVisitor) {
        Inst::get_operands(self, collector)
    }

    fn is_term(&self) -> MachTerminator {
        MachTerminator::None
    }

    fn is_trap(&self) -> bool {
        false
    }

    fn is_args(&self) -> bool {
        false
    }

    fn call_type(&self) -> CallType {
        CallType::None
    }

    fn is_mem_access(&self) -> bool {
        false
    }

    fn is_safepoint(&self) -> bool {
        true
    }
}

/// [`OperandVisitor`] extensions for [`Def`] and [`Use`]
pub trait SsaOperantVisitorExt: OperandVisitor {
    /// register a definition
    fn ssa_def(&mut self, reg: &mut Def) {
        self.add_operand(
            &mut reg.0,
            OperandConstraint::Reg,
            OperandKind::Def,
            OperandPos::Late,
        );
    }

    /// register a use
    fn ssa_use(&mut self, reg: &mut Use) {
        self.add_operand(
            &mut reg.0,
            OperandConstraint::Reg,
            OperandKind::Use,
            OperandPos::Early,
        );
    }
}

impl<T: OperandVisitor> SsaOperantVisitorExt for T {}

impl Inst {
    fn get_operands(&mut self, v: &mut impl OperandVisitor) {
        match self {
            MInst::Nop => {}
            MInst::Imm { d, .. } => v.ssa_def(d),
            MInst::AluRR { d, a, b, .. } => {
                v.ssa_def(d);
                v.ssa_use(a);
                v.ssa_use(b);
            }
            MInst::Return => {}
            MInst::Jump { args, .. } => {
                for arg in args {
                    v.ssa_use(arg);
                }
            }

            // since SSA IRs don't have physical register, these just forward their virtual ones
            MInst::Args { args } => {
                for arg in args {
                    v.reg_def(&mut arg.vreg)
                }
            }
            MInst::Rets { rets } => {
                for ret in rets {
                    v.reg_use(&mut ret.vreg)
                }
            }
        }
    }
}
