//! ISLE integration glue code for spirv lowering.

use crate::isa::spirv::SpirvBackend;
use crate::isa::spirv::lower::isle::generated_code::MInst;
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
use regalloc2::PReg;

pub mod generated_code;

type VecArgPair = Vec<ArgPair>;

pub(crate) struct SpirvIsleContext<'a, 'b, I, B>
where
    I: VCodeInst,
    B: LowerBackend,
{
    pub lower_ctx: &'a mut Lower<'b, I>,
    pub backend: &'a B,
}

impl<'a, 'b> SpirvIsleContext<'a, 'b, MInst, SpirvBackend> {
    fn new(lower_ctx: &'a mut Lower<'b, MInst>, backend: &'a SpirvBackend) -> Self {
        Self { lower_ctx, backend }
    }

    pub(crate) fn dfg(&self) -> &DataFlowGraph {
        &self.lower_ctx.f.dfg
    }
}

impl generated_code::Context for SpirvIsleContext<'_, '_, MInst, SpirvBackend> {
    isle_lower_prelude_methods!();

    #[inline]
    fn emit(&mut self, arg0: &MInst) -> Unit {
        self.lower_ctx.emit(arg0.clone());
    }

    #[inline]
    fn alloc_ssa(&mut self) -> Reg {
        self.lower_ctx.alloc_tmp(I32).only_reg().unwrap().to_reg()
    }
}

/// The main entry point for lowering with ISLE.
pub(crate) fn lower(
    lower_ctx: &mut Lower<MInst>,
    backend: &SpirvBackend,
    inst: Inst,
) -> Option<InstOutput> {
    let mut isle_ctx = SpirvIsleContext::new(lower_ctx, backend);
    generated_code::constructor_lower(&mut isle_ctx, inst)
}

/// The main entry point for branch lowering with ISLE.
pub(crate) fn lower_branch(
    lower_ctx: &mut Lower<MInst>,
    backend: &SpirvBackend,
    branch: Inst,
    targets: &[MachLabel],
) -> Option<()> {
    let mut isle_ctx = SpirvIsleContext::new(lower_ctx, backend);
    generated_code::constructor_lower_branch(&mut isle_ctx, branch, targets)
}
