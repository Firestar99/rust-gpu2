use crate::dominator_tree::DominatorTree;
use crate::ir::Function;
use crate::isa::spirv::inst::{EmitInfo, Inst, SpirvCallee};
use crate::isa::spirv::settings as spirv_settings;
use crate::isa::ssa::SsaAbi;
use crate::isa::unwind::systemv::RegisterMappingError;
use crate::isa::{Builder, FunctionAlignment, IsaFlagsHashKey, OwnedTargetIsa, TargetIsa};
use crate::machinst::{SigSet, VCode};
use crate::settings::{self as shared_settings};
use crate::{
    CodegenError, CodegenResult, CompiledCode, CompiledCodeStencil, MachTextSectionBuilder, Reg,
    TextSectionBuilder, ir,
};
use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::{boxed::Box, vec::Vec};
use cranelift_control::ControlPlane;
use target_lexicon::{Architecture, Triple};

mod inst;
mod lower;
mod settings;

/// A spirv backend.
pub struct SpirvBackend {
    triple: Triple,
    flags: shared_settings::Flags,
    isa_flags: spirv_settings::Flags,
}

impl SpirvBackend {
    /// Create a new spirv backend with the given (shared) flags.
    pub fn new_with_flags(
        triple: Triple,
        flags: shared_settings::Flags,
        isa_flags: spirv_settings::Flags,
    ) -> SpirvBackend {
        SpirvBackend {
            triple,
            flags,
            isa_flags,
        }
    }

    /// This performs lowering to VCode, register-allocates the code, computes block layout and
    /// finalizes branches. The result is ready for binary emission.
    fn compile_vcode(
        &self,
        func: &Function,
        domtree: &DominatorTree,
        regalloc_ctx: &mut regalloc2::Ctx,
        ctrl_plane: &mut ControlPlane,
    ) -> CodegenResult<VCode<Inst>> {
        let emit_info = EmitInfo::new(self.flags.clone(), self.isa_flags.clone());
        let sigs = SigSet::new::<SsaAbi<Inst>>(func, &self.flags)?;
        let abi = SpirvCallee::new(func, self, &self.isa_flags, &sigs)?;
        crate::machinst::compile::compile_ssa::<SpirvBackend>(
            func,
            domtree,
            regalloc_ctx,
            self,
            abi,
            emit_info,
            sigs,
            ctrl_plane,
        )
    }
}

impl TargetIsa for SpirvBackend {
    fn name(&self) -> &'static str {
        "spirv"
    }

    fn triple(&self) -> &Triple {
        &self.triple
    }

    fn flags(&self) -> &shared_settings::Flags {
        &self.flags
    }

    fn isa_flags(&self) -> Vec<shared_settings::Value> {
        self.isa_flags.iter().collect()
    }

    fn isa_flags_hash_key(&self) -> IsaFlagsHashKey<'_> {
        IsaFlagsHashKey(self.isa_flags.hash_key())
    }

    /// useless?
    fn dynamic_vector_bytes(&self, _dynamic_ty: ir::Type) -> u32 {
        16
    }

    fn compile_function(
        &self,
        func: &Function,
        domtree: &DominatorTree,
        regalloc_ctx: &mut regalloc2::Ctx,
        want_disasm: bool,
        ctrl_plane: &mut ControlPlane,
    ) -> CodegenResult<CompiledCodeStencil> {
        let vcode = self.compile_vcode(func, domtree, regalloc_ctx, ctrl_plane)?;

        let want_disasm = want_disasm || log::log_enabled!(log::Level::Debug);
        let emit_result = vcode.emit(&regalloc_ctx.output, want_disasm, &self.flags, ctrl_plane)?;
        let value_labels_ranges = emit_result.value_labels_ranges;
        let buffer = emit_result.buffer;

        if let Some(disasm) = emit_result.disasm.as_ref() {
            log::debug!("disassembly:\n{disasm}");
        }

        Ok(CompiledCodeStencil(CompiledCode {
            buffer,
            vcode: emit_result.disasm,
            value_labels_ranges,
            bb_starts: emit_result.bb_offsets,
            bb_edges: emit_result.bb_edges,
        }))
    }

    #[cfg(feature = "unwind")]
    fn map_regalloc_reg_to_dwarf(&self, _reg: Reg) -> Result<u16, RegisterMappingError> {
        Err(RegisterMappingError::UnsupportedArchitecture)
    }

    #[cfg(feature = "unwind")]
    fn emit_unwind_info(
        &self,
        _result: &CompiledCode,
        _kind: crate::isa::unwind::UnwindInfoKind,
    ) -> CodegenResult<Option<crate::isa::unwind::UnwindInfo>> {
        Err(CodegenError::Unsupported(
            "no unwinding on spirv".to_owned(),
        ))
    }

    #[cfg(feature = "unwind")]
    fn create_systemv_cie(&self) -> Option<gimli::write::CommonInformationEntry> {
        None
    }

    fn text_section_builder(&self, num_funcs: usize) -> Box<dyn TextSectionBuilder> {
        Box::new(MachTextSectionBuilder::<inst::Inst>::new(num_funcs))
    }

    fn function_alignment(&self) -> FunctionAlignment {
        FunctionAlignment {
            minimum: 1,
            preferred: 1,
        }
    }

    fn page_size_align_log2(&self) -> u8 {
        debug_assert_eq!(1 << 12, 0x1000);
        12
    }

    #[cfg(feature = "disas")]
    fn to_capstone(&self) -> Result<capstone::Capstone, capstone::Error> {
        Err(capstone::Error::UnsupportedArch)
    }

    fn pretty_print_reg(&self, reg: Reg, _size: u8) -> String {
        format!("{reg:?}")
    }

    fn has_native_fma(&self) -> bool {
        false
    }

    fn supports_vector_load_store(&self, _: ir::Type) -> bool {
        false
    }

    fn has_round(&self) -> bool {
        false
    }

    fn has_blendv_lowering(&self, _: ir::Type) -> bool {
        false
    }

    fn has_x86_pshufb_lowering(&self) -> bool {
        false
    }

    fn has_x86_pmulhrsw_lowering(&self) -> bool {
        false
    }

    fn has_x86_pmaddubsw_lowering(&self) -> bool {
        false
    }

    fn default_argument_extension(&self) -> ir::ArgumentExtension {
        ir::ArgumentExtension::None
    }
}

impl core::fmt::Display for SpirvBackend {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("MachBackend")
            .field("name", &self.name())
            .field("triple", &self.triple())
            .field("flags", &format!("{}", self.flags()))
            .finish()
    }
}

/// Create a new `isa::Builder`.
pub fn isa_builder(triple: Triple) -> Builder {
    match triple.architecture {
        Architecture::Spirv => {}
        _ => unreachable!(),
    }
    Builder {
        triple,
        setup: spirv_settings::builder(),
        constructor: isa_constructor,
    }
}

fn isa_constructor(
    triple: Triple,
    shared_flags: shared_settings::Flags,
    builder: &shared_settings::Builder,
) -> CodegenResult<OwnedTargetIsa> {
    let isa_flags = spirv_settings::Flags::new(&shared_flags, builder);
    let backend = SpirvBackend::new_with_flags(triple, shared_flags, isa_flags);
    Ok(backend.wrapped())
}
