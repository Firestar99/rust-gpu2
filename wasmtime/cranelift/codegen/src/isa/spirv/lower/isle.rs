//! ISLE integration glue code for spirv lowering.

use crate::isa::spirv::SpirvBackend;
use crate::isa::spirv::inst::{Use, VecUse};
use crate::isa::spirv::lower::isle::generated_code::{MInst, SpvLabel};
use crate::machinst::Reg;
use crate::machinst::{MachInst, isle::*};
use crate::machinst::{VCodeConstant, VCodeConstantData};
use crate::opts::{FloatCC, IntCC};
use crate::{
    ir::{
        BlockCall, ExternalName, Inst, InstructionData, MemFlagsData, Opcode, TrapCode, Value,
        ValueList, immediates::*, types::*,
    },
    machinst::{ArgPair, CallArgList, CallRetList, InstOutput},
};
use alloc::boxed::Box;
use alloc::vec::Vec;
use cranelift_entity::SecondaryMap;
use regalloc2::PReg;
use smallvec::SmallVec;
use crate::settings::Flags;

pub mod generated_code;

type VecArgPair = Vec<ArgPair>;

impl<'a> generated_code::Context for IsleContext<'_, '_, MInst, SpirvLowerBackend<'a>> {
    isle_lower_prelude_methods!();

    #[inline]
    fn emit(&mut self, arg0: &MInst) -> Unit {
        self.lower_ctx.emit(arg0.clone());
    }

    #[inline]
    fn alloc_ssa(&mut self) -> Reg {
        alloc_ssa(self.lower_ctx)
    }

    fn block_call_uses(&mut self, arg0: BlockCall) -> VecUse {
        arg0.args(&self.dfg().value_lists)
            .map(|arg| arg.as_value().expect("BlockCall with non-value arg"))
            .collect::<SmallVec<[_; 16]>>()
            .into_iter()
            // put_in_reg requires mutable access, so we can't hang on to `arg0.args()` Iterator and need to collect
            .map(|v| Use(self.put_in_reg(v)))
            .collect()
    }

    fn spv_label(&mut self, arg0: MachLabel) -> SpvLabel {
        self.backend.labels.get(arg0).unwrap().clone()
    }
}

fn alloc_ssa(lower: &mut Lower<MInst>) -> Reg {
    lower.alloc_tmp(I32).only_reg().unwrap().to_reg()
}

pub struct SpirvLowerBackend<'a> {
    pub b: &'a SpirvBackend,
    pub labels: SecondaryMap<MachLabel, SpvLabel>,
}

impl<'a> SpirvLowerBackend<'a> {
    pub fn new(b: &'a SpirvBackend, lower: &mut Lower<MInst>) -> Self {
        let num_blocks = lower.block_order().lowered_order().len();
        let labels = SecondaryMap::from(
            (0..num_blocks)
                .map(|i| SpvLabel(alloc_ssa(lower)))
                .collect::<Vec<_>>(),
        );
        Self { b, labels }
    }

    // required by isle_lower_prelude_methods!()
    pub fn flags(&self) -> &Flags {
        self.b.flags()
    }
}

impl LowerBackend for SpirvLowerBackend<'_> {
    type MInst = MInst;

    fn lower(&self, lower_ctx: &mut Lower<MInst>, inst: Inst) -> Option<InstOutput> {
        let mut isle_ctx = IsleContext {
            lower_ctx,
            backend: self,
        };
        generated_code::constructor_lower(&mut isle_ctx, inst)
    }

    fn lower_branch(
        &self,
        lower_ctx: &mut Lower<MInst>,
        branch: Inst,
        targets: &[MachLabel],
    ) -> Option<()> {
        let mut isle_ctx = IsleContext {
            lower_ctx,
            backend: self,
        };
        generated_code::constructor_lower_branch(&mut isle_ctx, branch, targets)
    }
}
