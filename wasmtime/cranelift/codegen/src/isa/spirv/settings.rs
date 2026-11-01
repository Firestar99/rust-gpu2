//! spirv Settings.

use crate::machinst::IsaFlags;
use crate::settings::{self, Builder, Value, detail};
use core::fmt;

include!(concat!(env!("OUT_DIR"), "/settings-spirv.rs"));

impl IsaFlags for Flags {}
