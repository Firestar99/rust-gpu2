use crate::{SmallValueVec, Waffle2Clif};
use cranelift_codegen::ir;
use cranelift_codegen::ir::{ExtFuncData, ExternalName, InstBuilder, Value};
use cranelift_frontend::FunctionBuilder;
use waffle::Operator;

pub struct LowerCall<F: Fn(waffle::Func) -> anyhow::Result<(ir::UserExternalName, ir::Signature)>>(
    pub F,
);

impl<F: Fn(waffle::Func) -> anyhow::Result<(ir::UserExternalName, ir::Signature)>> Waffle2Clif
    for LowerCall<F>
{
    fn lower(
        &self,
        builder: &mut FunctionBuilder,
        op: Operator,
        args: &[Value],
    ) -> anyhow::Result<Option<SmallValueVec>> {
        match op {
            Operator::Call { function_index } => {
                let (name, sig) = self.0(function_index)?;
                let name = builder.func.declare_imported_user_function(name);
                let sig = builder.import_signature(sig);
                let func_ref = builder.import_function(ExtFuncData {
                    name: ExternalName::User(name),
                    signature: sig,
                    colocated: false,
                    patchable: false,
                });
                let inst = builder.ins().call(func_ref, args);
                Ok(Some(builder.inst_results(inst).iter().copied().collect()))
            }
            _ => Ok(None),
        }
    }
}
