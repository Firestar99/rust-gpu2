use crate::isa::spirv::inst::{Def, SpvLabel, Use};
use core::fmt::{Display, Formatter};
use rspirv2::Word;
use rspirv2::operand::{IdRef, IdResult};

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
