use crate::{Waffle2Clif, lower_body, lower_sig};
use cranelift_codegen::ir;
use cranelift_codegen::isa::{CallConv, TargetFrontendConfig};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use std::ops::{Index, IndexMut};
use waffle::entity::EntityRef;

/// A `Waffle2Clif` module is a map from [`waffle::Func`] to their lowered clif [`ir::Function`].
///
/// Only functions with a [`waffle::FunctionBody`] are lowered, indexing an imported functions will return `None` or panic respectively.
pub struct Module {
    pub f2f: Vec<Option<ir::Function>>,
}

/// Options for waffle to cranelift translation
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct ModuleLowerOptions {
    /// The namespace id to use in [`UserExternalName::new`].
    pub clif_namespace: u32,
    /// The [`CallConv`] to use for translated functions
    pub call_conv: CallConv,
    /// [`TrapCode`] for reaching an unreachable
    pub unreachable_trap_code: ir::TrapCode,
    /// waffle optimization options, `None` disables waffle optimizations
    pub waffle_opt: Option<waffle::OptOptions>,
}

impl ModuleLowerOptions {
    pub fn user_func_name(&self, func: waffle::Func) -> ir::UserExternalName {
        ir::UserExternalName {
            namespace: self.clif_namespace,
            index: func.index() as u32,
        }
    }
}

impl Default for ModuleLowerOptions {
    fn default() -> Self {
        Self {
            clif_namespace: 0,
            call_conv: CallConv::Fast,
            unreachable_trap_code: ir::TrapCode::user(1).unwrap(),
            waffle_opt: Some(waffle::OptOptions::default()),
        }
    }
}

impl Module {
    /// Lower all functions from a [`waffle::Module`] to clif, returning a waffle2clif [`Module`].
    ///
    /// Retrieve the [`TargetFrontendConfig`] by first constructing a cranelift `&dyn`[`TargetIsa`] using the
    /// `cranelift_codegen::isa::<isa>::isa_builder` builder of your choice, then call `frontend_config()` on it.
    ///
    /// [`TargetIsa`]: cranelift_codegen::isa::TargetIsa
    pub fn lower(
        module: &mut waffle::Module,
        lower: impl Waffle2Clif,
        frontend_config: TargetFrontendConfig,
        opt: &ModuleLowerOptions,
    ) -> anyhow::Result<Self> {
        log::trace!("lowering unexpanded waffle module: \n{}", module.display());

        let mut bdr_ctx = FunctionBuilderContext::default();
        let f2f = (0..module.funcs.len() as u32)
            .map(|func| {
                let func = waffle::Func::from(func);
                {
                    // waffle func expansion and optimization
                    let func_decl = module.expand_func(func)?;
                    if let Some(waffle_opts) = &opt.waffle_opt
                        && let Some(body) = func_decl.body_mut()
                    {
                        body.optimize(waffle_opts);
                    }
                }

                // lowering, reborrow to allow accessing other functions to lower calls
                let func_decl = &module.funcs[func];
                if let Some(body) = func_decl.body() {
                    log::debug!("lowering function `{}`", func_decl.name());
                    log::trace!(
                        "function `{}` waffle:\n{}",
                        func_decl.name(),
                        body.display_verbose("| ", None)
                    );
                    let sig = lower_sig(&module, module.funcs[func].sig(), opt.call_conv)?;
                    let mut func_c = ir::Function::with_name_signature(
                        ir::UserFuncName::User(opt.user_func_name(func)),
                        sig.clone(),
                    );
                    let mut bdr = FunctionBuilder::new(&mut func_c, &mut bdr_ctx);
                    lower_body(body, &mut bdr, &lower, opt.unreachable_trap_code)?;
                    bdr.finalize(frontend_config);
                    log::trace!(
                        "function `{}` clif:\n{}",
                        func_decl.name(),
                        func_c.display()
                    );
                    Ok(Some(func_c))
                } else {
                    log::debug!("imported function `{}` not lowered", func_decl.name());
                    Ok(None)
                }
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(Self { f2f })
    }

    pub fn get(&self, index: waffle::Func) -> Option<&ir::Function> {
        self.f2f.get(index.index()).map(Option::as_ref).flatten()
    }

    pub fn get_mut(&mut self, index: waffle::Func) -> Option<&mut ir::Function> {
        self.f2f
            .get_mut(index.index())
            .map(Option::as_mut)
            .flatten()
    }
}

impl Index<waffle::Func> for Module {
    type Output = ir::Function;

    fn index(&self, index: waffle::Func) -> &Self::Output {
        self.get(index).unwrap()
    }
}

impl IndexMut<waffle::Func> for Module {
    fn index_mut(&mut self, index: waffle::Func) -> &mut Self::Output {
        self.get_mut(index).unwrap()
    }
}
