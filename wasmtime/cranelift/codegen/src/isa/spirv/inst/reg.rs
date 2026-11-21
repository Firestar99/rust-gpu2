use crate::Reg;
use crate::isa::spirv::inst::{Def, SpvLabel, Use};
use crate::isa::ssa::SSA_REG_CLASS;
use core::fmt::{Display, Formatter};
use regalloc2::VReg;
use rspirv2::operand::{IdRef, IdResult};

impl Def {
    /// create a new Def of some index
    #[inline]
    pub fn new(index: usize) -> Self {
        Self(Reg::from_virtual_reg(VReg::new(index, SSA_REG_CLASS)))
    }

    /// virtual register index of this Def
    #[inline]
    pub fn index(&self) -> u32 {
        self.0.to_virtual_reg().unwrap().index() as u32
    }

    /// convert to `rspirv2`'s [`IdResult`]
    #[inline]
    pub fn to_rspirv2(&self) -> IdResult {
        IdResult(self.index())
    }
}

impl Display for Def {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "%{}", self.index())
    }
}

impl Use {
    /// create a new Use of some index
    #[inline]
    pub fn new(index: usize) -> Self {
        Self(Reg::from_virtual_reg(VReg::new(index, SSA_REG_CLASS)))
    }

    /// virtual register index of this Use
    #[inline]
    pub fn index(&self) -> u32 {
        self.0.to_virtual_reg().unwrap().index() as u32
    }

    /// convert to `rspirv2`'s [`IdResult`]
    #[inline]
    pub fn to_rspirv2(&self) -> IdRef {
        IdRef(IdResult(self.index()))
    }
}

impl Display for Use {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "%{}", self.index())
    }
}

/// The bit indicating this is a SpvLabel Register for `OpLabel`s.
///
/// Must be below `REG_SPILLSLOT_BIT >> 2` or [`Reg::from_virtual_reg`] will panic.
pub const SPIRV_REG_LABEL_BIT: u32 = 0x1000_0000;

impl SpvLabel {
    /// create a new SpvLabel of some index
    #[inline]
    pub fn new(index: usize) -> Self {
        Self(Reg::from_virtual_reg(VReg::new(
            index | SPIRV_REG_LABEL_BIT as usize,
            SSA_REG_CLASS,
        )))
    }

    /// virtual register index of this SpvLabel
    #[inline]
    pub fn id_result_index(&self) -> u32 {
        self.0.to_virtual_reg().unwrap().index() as u32
    }

    /// virtual register index of this SpvLabel
    #[inline]
    pub fn index(&self) -> u32 {
        self.id_result_index() & !SPIRV_REG_LABEL_BIT
    }

    /// convert to `rspirv2`'s [`IdResult`]
    #[inline]
    pub fn to_rspirv2(&self) -> IdResult {
        IdResult(self.id_result_index())
    }
}

// TODO ew
impl Default for SpvLabel {
    fn default() -> Self {
        SpvLabel::new(0)
    }
}
