use crate::Module;
use cranelift_codegen::inline::{Inline, InlineCommand};
use cranelift_codegen::ir;
use cranelift_codegen::ir::ExternalName;
use std::borrow::Cow;

pub struct ExhaustiveInliner<'a> {
    /// A waffle2clif Module
    pub module: &'a Module,
    /// The [`ir::UserExternalName`]`.namespace` that a function must be to be inlined.
    pub namespace: u32,
}

impl Inline for ExhaustiveInliner<'_> {
    fn inline(
        &mut self,
        caller: &ir::Function,
        _call_inst: ir::Inst,
        _call_opcode: ir::Opcode,
        callee: ir::FuncRef,
        _call_args: &[ir::Value],
    ) -> InlineCommand<'_> {
        match caller.stencil.dfg.ext_funcs[callee].name {
            ExternalName::User(name_ref) => {
                let func = caller.params.user_named_funcs()[name_ref].clone();
                if func.namespace == self.namespace {
                    match self.module.get(waffle::Func::from(func.index)) {
                        Some(func) => {
                            return InlineCommand::Inline {
                                callee: Cow::Borrowed(func),
                                visit_callee: true,
                            };
                        }
                        None => (),
                    }
                }
            }
            _ => (),
        }
        InlineCommand::KeepCall
    }
}
