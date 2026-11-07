use crate::ir::UserStackMap;
use crate::isa::spirv::inst::ty::dis_context;
use crate::isa::spirv::inst::{Def, Inst, SpirvCallee};
use crate::isa::spirv::lower::isle::generated_code::{AluOpRR, MInst, TypeInt};
use crate::{FrameLayout, MachBuffer, MachInstEmit, MachInstEmitState, VCodeInst, settings};
use alloc::string::String;
use core::fmt::Write;
use cranelift_control::ControlPlane;
use rspirv2::Word;
use rspirv2::binary::WordWriter;
use rspirv2::core::inst::{OpBranch, OpConstant, OpIAdd, OpIMul, OpISub, OpNop, OpReturn};
use rspirv2::core::inst_set::CoreInstSet;
use rspirv2::inst::InstEncoding;
use rspirv2::operand::{IdRef, LiteralConst};

/// spirv EmitInfo
pub struct EmitInfo {
    #[expect(dead_code, reason = "may want to be used in the future")]
    shared_flag: settings::Flags,
    #[expect(dead_code, reason = "may want to be used in the future")]
    isa_flags: super::super::spirv_settings::Flags,
}

impl EmitInfo {
    pub(crate) fn new(
        shared_flag: settings::Flags,
        isa_flags: super::super::spirv_settings::Flags,
    ) -> Self {
        Self {
            shared_flag,
            isa_flags,
        }
    }
}

/// State carried between emissions of a sequence of instructions.
#[derive(Default, Clone, Debug)]
pub struct EmitState {
    ctrl_plane: ControlPlane,
    frame_layout: FrameLayout,
}

impl MachInstEmitState<Inst> for EmitState {
    fn new(abi: &SpirvCallee, ctrl_plane: ControlPlane) -> Self {
        Self {
            ctrl_plane,
            frame_layout: abi.frame_layout().clone(),
        }
    }

    fn pre_safepoint(&mut self, _: Option<UserStackMap>) {}

    fn ctrl_plane_mut(&mut self) -> &mut ControlPlane {
        &mut self.ctrl_plane
    }

    fn take_ctrl_plane(self) -> ControlPlane {
        self.ctrl_plane
    }

    fn frame_layout(&self) -> &FrameLayout {
        &self.frame_layout
    }
}

impl MachInstEmit for Inst {
    type State = EmitState;
    type Info = EmitInfo;

    fn emit(&self, code: &mut MachBuffer<Self>, _info: &Self::Info, _state: &mut Self::State) {
        if let Some(inst) = self.to_rspirv2() {
            inst.encode(&mut MachWordWriter(code)).unwrap()
        }
    }

    fn pretty_print_inst(&self, _state: &mut Self::State) -> String {
        match self {
            MInst::Args { args } => {
                let mut f = String::from("Args { ");
                for (i, arg) in args.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ").unwrap();
                    }
                    write!(f, "{}", Def(arg.vreg.to_reg())).unwrap();
                }
                write!(f, " }}").unwrap();
                f
            }
            MInst::Rets { rets } => {
                let mut f = String::from("Rets { ");
                for (i, ret) in rets.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ").unwrap();
                    }
                    write!(f, "{}", Def(ret.vreg)).unwrap();
                }
                write!(f, " }}").unwrap();
                f
            }
            _ => self
                .to_rspirv2()
                .map_or_default(|inst| format!("{}", inst.dis(&dis_context()))),
        }
    }
}

impl Inst {
    /// convert this instruction to `rspirv2`'s [`CoreInstSet`]
    pub fn to_rspirv2(&self) -> Option<CoreInstSet> {
        Some(match self {
            MInst::Nop => OpNop {}.into(),
            MInst::Imm { ty, d, imm } => OpConstant {
                id_result_type: ty.to_rspirv2(),
                id_result: d.to_rspirv2(),
                value: match ty {
                    TypeInt::I8 | TypeInt::I16 | TypeInt::I32 => LiteralConst::from(*imm as u32),
                    TypeInt::I64 => LiteralConst::from(*imm),
                },
            }
            .into(),
            MInst::AluRR { ty, op, d, a, b } => {
                let id_result = d.to_rspirv2();
                let id_result_type = ty.to_rspirv2();
                let operand_1 = a.to_rspirv2();
                let operand_2 = b.to_rspirv2();
                match op {
                    AluOpRR::IAdd => OpIAdd {
                        id_result_type,
                        id_result,
                        operand_1,
                        operand_2,
                    }
                    .into(),
                    AluOpRR::ISub => OpISub {
                        id_result_type,
                        id_result,
                        operand_1,
                        operand_2,
                    }
                    .into(),
                    AluOpRR::IMul => OpIMul {
                        id_result_type,
                        id_result,
                        operand_1,
                        operand_2,
                    }
                    .into(),
                }
            }
            MInst::Return => OpReturn {}.into(),
            MInst::Jump { label, args } => {
                // TODO args?
                let _ = args;
                OpBranch {
                    target_label: IdRef(label.to_rspirv2()),
                }
                .into()
            }
            MInst::Args { .. } | MInst::Rets { .. } => return None,
        })
    }
}

pub struct MachWordWriter<'a, T: VCodeInst>(pub &'a mut MachBuffer<T>);

impl<'a, T: VCodeInst> WordWriter for MachWordWriter<'a, T> {
    #[inline]
    fn write(&mut self, word: Word) {
        self.0.put4(word.0)
    }

    #[inline]
    fn write_iter(&mut self, iter: impl IntoIterator<Item = Word>) {
        for word in iter {
            self.0.put4(word.0)
        }
    }
}
