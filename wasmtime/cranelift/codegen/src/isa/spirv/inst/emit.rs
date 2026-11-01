use crate::ir::UserStackMap;
use crate::isa::spirv::inst::{Inst, SpirvCallee};
use crate::{FrameLayout, MachBuffer, MachInstEmit, MachInstEmitState, settings};
use cranelift_control::ControlPlane;
use alloc::string::String;

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

    fn emit(&self, code: &mut MachBuffer<Self>, info: &Self::Info, state: &mut Self::State) {
        todo!()
    }

    fn pretty_print_inst(&self, state: &mut Self::State) -> String {
        todo!()
    }
}
