use crate::logical_layout::make_logical_layout_module;
use anyhow::Context;
use cranelift_codegen::settings;
use cranelift_codegen::settings::Configurable;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use rspirv2::core::inst_set::CoreInstSet;
use rspirv2::dis::DisOptions;
use rspirv2::module::Module;
use rspirv2::vec::InstVec;
use std::collections::HashSet;
use target_lexicon::{Architecture, Triple};
use waffle2clif::{Compute, lower_body, lower_func};

mod source;
pub use source::*;
pub mod logical_layout;

/// The wasm to spirv compiler
pub struct Compiler<S: Source> {
    /// Source of the wasm module
    pub source: S,
    /// [`waffle`] frontend options
    pub waffle_frontend_opts: waffle::FrontendOptions,
    /// [`waffle`] optimization options
    pub waffle_opt_opts: waffle::OptOptions,
    /// [`waffle2clif`] lower options
    pub waffle2clif_lower_opts: waffle2clif::LowerOptions,
    /// A list of entry point names that should be compiled
    pub entry_points: HashSet<String>,
}

/// A spirv module of the compiler invocation
#[derive(Clone, Debug)]
pub struct SpirvModule {
    /// The name of the entry point function
    pub name: String,
    /// The spirv module
    pub module: Module<CoreInstSet>,
}

impl<S: Source> Compiler<S> {
    pub fn new(source: S) -> Self {
        Self {
            source,
            waffle_frontend_opts: waffle::FrontendOptions::default(),
            waffle_opt_opts: waffle::OptOptions::default(),
            waffle2clif_lower_opts: waffle2clif::LowerOptions::default(),
            entry_points: HashSet::new(),
        }
    }

    pub fn add_entry_point(&mut self, name: impl Into<String>) {
        self.entry_points.insert(name.into());
    }

    pub fn compile(self) -> anyhow::Result<Vec<SpirvModule>> {
        log::info!("compiling wasm to spirv");
        let wasm_bytes = self.source.compile()?;

        let mut module = waffle::Module::from_wasm_bytes(&wasm_bytes, &self.waffle_frontend_opts)?;
        module.expand_all_funcs()?;
        module.per_func_body(|body| body.optimize(&self.waffle_opt_opts));

        let mut bdr_ctx = FunctionBuilderContext::default();
        module
            .funcs
            .entries()
            .filter(|(_, d)| self.entry_points.contains(d.name()))
            .map(|(func, func_decl)| {
                log::info!("compiling function {}:", func_decl.name());

                // waffle2clif
                let func_c = {
                    let body = func_decl
                        .body()
                        .with_context(|| format!("Function must have a body: {:?}", func_decl))?;
                    log::debug!("waffle:\n{}", body.display("", None));

                    let mut func_c = lower_func(&module, func, &self.waffle2clif_lower_opts)?;
                    let mut bdr = FunctionBuilder::new(&mut func_c, &mut bdr_ctx);
                    lower_body(body, &mut bdr, Compute, &self.waffle2clif_lower_opts)?;

                    log::debug!("CLIF:\n{}", func_c.display(),);
                    func_c
                };

                // cranelift
                let mut comp_ctx = cranelift_codegen::Context::for_function(func_c);
                let compiled_code = {
                    // comp_ctx.set_disasm(true);
                    let mut settings = settings::builder();
                    settings.set("opt_level", "speed")?;
                    let isa = cranelift_codegen::isa::spirv::isa_builder(Triple {
                        architecture: Architecture::Spirv,
                        ..Triple::unknown()
                    })
                    .finish(settings::Flags::new(settings))?;
                    comp_ctx
                        .compile(&*isa, &mut Default::default())
                        .map_err(|e| e.inner)?
                };

                // to rspirv2::InstVec
                let inst_vec = {
                    let (chunks, rest) = compiled_code.buffer.data().as_chunks::<4>();
                    assert_eq!(rest.len(), 0);
                    let inst_vec = InstVec::<CoreInstSet>::from_words_unchecked(
                        chunks.iter().copied().map(u32::from_ne_bytes).collect(),
                    );
                    log::debug!("spirv:\n{}", inst_vec.dis(DisOptions::default()));
                    inst_vec
                };

                let module = make_logical_layout_module(inst_vec);
                Ok(SpirvModule {
                    name: func_decl.name().to_string(),
                    module,
                })
            })
            .collect()
    }
}
