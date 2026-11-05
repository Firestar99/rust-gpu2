use anyhow::Context;
use std::fmt::Debug;
use std::path::PathBuf;

#[cfg(feature = "cargo")]
mod cargo;
#[cfg(feature = "cargo")]
pub use cargo::*;

/// Where does the wasm blob come from?
pub trait Source: Debug {
    fn compile(self) -> anyhow::Result<Vec<u8>>;
}

/// Wasm blob is a slice in memory
impl Source for &[u8] {
    fn compile(self) -> anyhow::Result<Vec<u8>> {
        Ok(self.to_vec())
    }
}

/// Wasm blob loaded from a file
#[derive(Clone, Debug)]
pub struct FileSource(pub PathBuf);

impl Source for &FileSource {
    fn compile(self) -> anyhow::Result<Vec<u8>> {
        Ok(std::fs::read(&self.0)
            .with_context(|| format!("File not found: {}", self.0.display()))?)
    }
}
