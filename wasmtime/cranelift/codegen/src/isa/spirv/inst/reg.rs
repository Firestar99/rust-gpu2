use crate::isa::spirv::inst::{Def, SpvLabel, Use};
use core::fmt::{Display, Formatter};
use regalloc2::VReg;
use rspirv2::Word;
use rspirv2::operand::{IdRef, IdResult};
use crate::isa::ssa::SSA_REG_CLASS;
use crate::Reg;

impl Def {
    /// virtual register index of this Def
    #[inline]
    pub fn index(&self) -> u32 {
        self.0.to_virtual_reg().unwrap().index() as u32
    }

    /// convert to `rspirv2`'s [`IdResult`]
    #[inline]
    pub fn to_rspirv2(&self) -> IdResult {
        IdResult(Word(self.index()))
    }
}

impl Display for Def {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "%{}", self.index())
    }
}

impl Use {
    /// virtual register index of this Use
    #[inline]
    pub fn index(&self) -> u32 {
        self.0.to_virtual_reg().unwrap().index() as u32
    }

    /// convert to `rspirv2`'s [`IdResult`]
    #[inline]
    pub fn to_rspirv2(&self) -> IdRef {
        IdRef(IdResult(Word(self.index())))
    }
}

impl Display for Use {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "%{}", self.index())
    }
}

impl SpvLabel {
    /// virtual register index of this Use
    #[inline]
    pub fn index(&self) -> u32 {
        self.0.to_virtual_reg().unwrap().index() as u32
    }

    /// convert to `rspirv2`'s [`IdResult`]
    #[inline]
    pub fn to_rspirv2(&self) -> IdResult {
        IdResult(Word(self.index()))
    }
}

// TODO ew
impl Default for SpvLabel {
    fn default() -> Self {
        SpvLabel(Reg::from_virtual_reg(VReg::new(0, SSA_REG_CLASS)))
    }
}
