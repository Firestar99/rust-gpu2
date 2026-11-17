use rspirv2::dis::DisOptions;
use wasmsl::CargoSource;

pub fn main() -> anyhow::Result<()> {
    env_logger::init();

    let shader_crate = "basic-compute-shader";
    let mut compiler = wasmsl::Compiler::new(CargoSource::new(shader_crate));
    compiler.add_entry_point("bla");
    let modules = compiler.compile()?;

    for module in modules {
        println!(
            "Disassembly of Module {}:\n{}",
            module.name,
            module.module.dis(DisOptions::default())
        );

        let mut bytes = Vec::new();
        module.module.write_bytes(&mut bytes)?;
        std::fs::write(
            format!("{}/dump/{}.spv", env!("CARGO_MANIFEST_DIR"), module.name),
            bytes,
        )?;
    }
    Ok(())
}
