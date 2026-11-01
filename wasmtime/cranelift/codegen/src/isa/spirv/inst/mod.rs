pub use crate::isa::spirv::inst::emit::EmitInfo;
pub use crate::isa::spirv::lower::isle::generated_code::{Def, MInst as Inst, Use};
use crate::isa::ssa::{SsaAbi, SsaInst};
use crate::machinst::{
    ArgPair, CallType, Callee, MachTerminator, OperandVisitor, OperandVisitorImpl, RetPair,
    SmallInstVec,
};
use alloc::vec::Vec;
use regalloc2::{OperandConstraint, OperandKind, OperandPos};

mod emit;
mod ty;

pub type SpirvCallee = Callee<SsaAbi<Inst>>;

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
        match self {
            Inst::Return | Inst::Rets { .. } => MachTerminator::Ret,
            _ => MachTerminator::None,
        }
    }

    fn is_trap(&self) -> bool {
        false
    }

    fn is_args(&self) -> bool {
        matches!(self, Inst::Args {..})
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

pub trait SsaOperantVisitorExt: OperandVisitor {
    fn ssa_def(&mut self, reg: &mut Def) {
        self.add_operand(
            &mut reg.0,
            OperandConstraint::Reg,
            OperandKind::Def,
            OperandPos::Late,
        );
    }

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
            Inst::Nop => {}
            Inst::Imm { d, .. } => v.ssa_def(d),
            Inst::AluRR { d, a, b, .. } => {
                v.ssa_def(d);
                v.ssa_use(a);
                v.ssa_use(b);
            }
            Inst::Return => {}

            // since SSA IRs don't have physical register, these just forward their virtual ones
            Inst::Args { args } => {
                for arg in args {
                    v.reg_def(&mut arg.vreg)
                }
            }
            Inst::Rets { rets } => {
                for ret in rets {
                    v.reg_use(&mut ret.vreg)
                }
            }
        }
    }
}
