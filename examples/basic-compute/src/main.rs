use wasmsl::CargoSource;

pub fn main() -> anyhow::Result<()> {
    env_logger::init();

    let shader_crate = "basic-compute-shader";
    let mut compiler = wasmsl::Compiler::new(CargoSource::new(shader_crate));
    compiler.add_entry_point("bla");
    compiler.compile()?;
    Ok(())
}
