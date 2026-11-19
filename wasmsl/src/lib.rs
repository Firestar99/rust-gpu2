use crate::logical_layout::make_logical_layout_module;
use cranelift_codegen::settings;
use cranelift_codegen::settings::Configurable;
use rspirv2::core::inst_set::CoreInstSet;
use rspirv2::dis::DisOptions;
use rspirv2::module::Module;
use rspirv2::vec::InstVec;
use std::collections::HashSet;
use target_lexicon::{Architecture, Triple};
use waffle2clif::ExhaustiveInliner;

mod source;
pub use source::*;
pub mod logical_layout;

/// The wasm to spirv compiler
pub struct Compiler<S: Source> {
    /// Source of the wasm module
    pub source: S,
    /// [`waffle`] frontend options
    pub waffle_frontend_opts: waffle::FrontendOptions,
    /// [`waffle2clif`] lower options
    pub waffle2clif_opts: waffle2clif::ModuleLowerOptions,
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
            waffle2clif_opts: waffle2clif::ModuleLowerOptions::default(),
            entry_points: HashSet::new(),
        }
    }

    pub fn add_entry_point(&mut self, name: impl Into<String>) {
        self.entry_points.insert(name.into());
    }

    pub fn compile(self) -> anyhow::Result<Vec<SpirvModule>> {
        log::info!("compiling wasm to spirv");
        let mut settings = settings::builder();
        settings.set("opt_level", "speed")?;
        let isa = cranelift_codegen::isa::spirv::isa_builder(Triple {
            architecture: Architecture::Spirv,
            ..Triple::unknown()
        })
        .finish(settings::Flags::new(settings))?;

        let wasm_bytes = self.source.compile()?;
        let mut w_module =
            waffle::Module::from_wasm_bytes(&wasm_bytes, &self.waffle_frontend_opts)?;
        let module = waffle2clif::Module::lower(
            &mut w_module,
            waffle2clif::Compute,
            isa.frontend_config(),
            &self.waffle2clif_opts,
        )?;

        let mut cg_ctx = cranelift_codegen::Context::new();
        w_module
            .funcs
            .entries()
            .filter(|(_, d)| self.entry_points.contains(d.name()))
            .map(|(func, func_decl)| {
                log::info!("compiling entry point `{}` to spv", func_decl.name());

                // cranelift compile
                let compiled_code = {
                    cg_ctx.clear();
                    cg_ctx.func = module[func].clone();
                    log::trace!(
                        "entry point `{}` clif:\n{}",
                        func_decl.name(),
                        cg_ctx.func.display()
                    );
                    cg_ctx.inline(ExhaustiveInliner {
                        module: &module,
                        namespace: self.waffle2clif_opts.clif_namespace,
                    })?;
                    log::trace!(
                        "entry point `{}` clif after inlining:\n{}",
                        func_decl.name(),
                        cg_ctx.func.display()
                    );
                    cg_ctx
                        .compile(&*isa, &mut Default::default())
                        .map_err(|e| e.inner)?;
                    cg_ctx.take_compiled_code().unwrap()
                };

                // to rspirv2::InstVec
                let inst_vec = {
                    let (chunks, rest) = compiled_code.buffer.data().as_chunks::<4>();
                    assert_eq!(rest.len(), 0);
                    let inst_vec = InstVec::<CoreInstSet>::from_words_unchecked(
                        chunks.iter().copied().map(u32::from_ne_bytes).collect(),
                    );
                    log::trace!(
                        "entry point `{}` spirv:\n{}",
                        func_decl.name(),
                        inst_vec.dis(DisOptions {
                            color: false,
                            ..DisOptions::default()
                        })
                    );
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
