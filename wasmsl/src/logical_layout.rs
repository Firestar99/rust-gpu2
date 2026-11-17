//! Create a SPIR-V module that follows the "logical layout" rules, required for the shader module to be valid.
//!
//! See "2.4. Logical Layout of a Module"
//! <https://registry.khronos.org/SPIR-V/specs/unified1/SPIRV.html#LogicalLayout>

use cranelift_codegen::isa::spirv::inst::SPIRV_VOID;
use rspirv2::core::inst::{
    OpCapability, OpEntryPoint, OpExecutionMode, OpFunction, OpFunctionEnd, OpLabel, OpMemoryModel,
    OpTypeFunction,
};
use rspirv2::core::inst_set::CoreInstSet;
use rspirv2::core::operands::{
    AddressingModel, Capability, ExecutionMode, ExecutionModel, MemoryModel,
};
use rspirv2::module::{Module, SpirvHeader, SpirvVersion};
use rspirv2::operand::{IdRef, IdResult, LiteralInteger, LiteralString};
use rspirv2::vec::InstVec;

/// id of the entry point function
///
/// **Must not collide with preamble or emission!**
pub const ENTRY_POINT_ID: IdResult = IdResult(100);
pub const TYPE_FUNCTION_VOID: IdResult = IdResult(101);
pub const ENTRY_POINT_LABEL: IdResult = IdResult(102);

/// merge parts to make a proper module with the required logical layout
pub fn make_logical_layout_module(fn_inst: InstVec<CoreInstSet>) -> Module<CoreInstSet> {
    let mut vec = InstVec::new();
    vec.push_inst(OpCapability {
        capability: Capability::Shader,
    });
    vec.push_inst(OpCapability {
        capability: Capability::Int8,
    });
    vec.push_inst(OpCapability {
        capability: Capability::Int16,
    });
    vec.push_inst(OpCapability {
        capability: Capability::Int64,
    });
    vec.push_inst(OpMemoryModel {
        addressing_model: AddressingModel::Logical,
        memory_model: MemoryModel::Simple,
    });
    vec.push_inst(OpEntryPoint {
        execution_model: ExecutionModel::GLCompute,
        entry_point: IdRef(ENTRY_POINT_ID),
        name: LiteralString("main".to_string()),
        interface: Default::default(),
    });
    vec.push_inst(OpExecutionMode {
        entry_point: IdRef(ENTRY_POINT_ID),
        mode: ExecutionMode::LocalSize(
            LiteralInteger::new(1),
            LiteralInteger::new(1),
            LiteralInteger::new(1),
        ),
    });
    cranelift_codegen::isa::spirv::inst::push_preamble(&mut vec);
    let type_function = vec.push_inst(OpTypeFunction {
        id_result: TYPE_FUNCTION_VOID,
        return_type: IdRef(SPIRV_VOID.0),
        id_ref: Default::default(),
    });
    vec.extend(
        fn_inst
            .iter()
            .filter(|inst| matches!(inst, CoreInstSet::Constant(..))),
    );
    vec.push_inst(OpFunction {
        id_result_type: SPIRV_VOID,
        id_result: ENTRY_POINT_ID,
        function_control: Default::default(),
        function_type: IdRef(type_function),
    });
    vec.push_inst(OpLabel {
        id_result: ENTRY_POINT_LABEL,
    });
    vec.extend(
        fn_inst
            .iter()
            .filter(|inst| !matches!(inst, CoreInstSet::Constant(..))),
    );
    vec.push_inst(OpFunctionEnd {});

    // TODO with rspirv2 you can't trivially figure this one out
    let highest_result_id = 8192;
    Module {
        header: Some(SpirvHeader::new(
            SpirvVersion::new(1, 6),
            11,
            highest_result_id,
        )),
        inst: vec,
    }
}
