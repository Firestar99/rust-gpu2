use anyhow::Context;
use cranelift_codegen::settings;
use cranelift_codegen::settings::Configurable;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use rspirv2::Word;
use rspirv2::core::inst_set::CoreInstSet;
use rspirv2::dis::DisOptions;
use rspirv2::vec::InstVec;
use target_lexicon::{Architecture, Triple};
use waffle::FrontendOptions;
use waffle2cranelift::{Compute, LowerOptions, lower_body, lower_func};

pub fn main() -> anyhow::Result<()> {
    let shader_crate = "basic_arithmic_shader";
    let target = "wasm32-unknown-unknown";
    let opts = LowerOptions::default();

    let status = std::process::Command::new("cargo")
        .args([
            "rustc",
            "-p",
            shader_crate,
            "--target",
            target,
            "--release",
            "--crate-type",
            "cdylib",
            // "--message-format",
            // "json",
        ])
        .status()?;
    anyhow::ensure!(status.success());

    let artifact_path = format!(
        "{}/../../target/{target}/release/{shader_crate}.wasm",
        env!("CARGO_MANIFEST_DIR")
    );
    let wasm_bytes = std::fs::read(&artifact_path)
        .with_context(|| format!("Artifact not found at path {artifact_path}"))?;

    let mut module = waffle::Module::from_wasm_bytes(&wasm_bytes, &FrontendOptions::default())?;
    module.expand_all_funcs()?;

    let mut bdr_ctx = FunctionBuilderContext::default();
    for (func, func_decl) in module.funcs.entries() {
        println!("Function {}:", func_decl.name());

        let func_c = {
            let body = func_decl
                .body()
                .with_context(|| format!("Function must have a body: {:?}", func_decl))?;
            println!("wasm:\n{}", body.display("", None));

            let mut func_c = lower_func(&module, func, &opts)?;
            let mut bdr = FunctionBuilder::new(&mut func_c, &mut bdr_ctx);
            lower_body(body, &mut bdr, Compute, &opts)?;

            println!("CLIF:\n{}", func_c.display(),);
            func_c
        };

        let mut comp_ctx = cranelift_codegen::Context::for_function(func_c);
        // comp_ctx.set_disasm(true);
        let mut settings = settings::builder();
        settings.set("opt_level", "speed")?;
        let isa = cranelift_codegen::isa::spirv::isa_builder(Triple {
            architecture: Architecture::Spirv,
            ..Triple::unknown()
        })
        .finish(settings::Flags::new(settings))?;
        let compiled_code = comp_ctx
            .compile(&*isa, &mut Default::default())
            .map_err(|e| e.inner)?;

        let (chunks, rest) = compiled_code.buffer.data().as_chunks::<4>();
        assert_eq!(rest.len(), 0);
        let inst_vec = InstVec::<CoreInstSet>::from_words_unchecked(
            chunks
                .iter()
                .map(|a| Word(u32::from_ne_bytes(*a)))
                .collect(),
        );
        println!("spirv:\n{}\n\n\n", inst_vec.dis(DisOptions::default()),);
    }

    Ok(())
}
