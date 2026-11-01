use crate::isa::spirv::lower::isle::generated_code::TypeInt;
use rspirv2::Word;
use rspirv2::core::inst::{OpTypeBool, OpTypeInt, OpTypeVoid};
use rspirv2::core::inst_set::CoreInstSet;
use rspirv2::dis::{DisContext, DisOptions};
use rspirv2::operand::{IdResult, IdResultType, LiteralInteger};
use rspirv2::vec::InstVec;

/// The id of the [`OpTypeVoid`] declaration in the [`preamble`]
pub const SPIRV_VOID: IdResultType = IdResultType(IdResult(Word(0)));

/// The id of the [`OpTypeBool`] declaration in the [`preamble`]
pub const SPIRV_BOOL: IdResultType = IdResultType(IdResult(Word(1)));
/// The id of the `i8` declaration in the [`preamble`]
pub const SPIRV_I8: IdResultType = IdResultType(IdResult(Word(2)));
/// The id of the `i16` declaration in the [`preamble`]
pub const SPIRV_I16: IdResultType = IdResultType(IdResult(Word(3)));
/// The id of the `i32` declaration in the [`preamble`]
pub const SPIRV_I32: IdResultType = IdResultType(IdResult(Word(4)));
/// The id of the `i64` declaration in the [`preamble`]
pub const SPIRV_I64: IdResultType = IdResultType(IdResult(Word(5)));

impl TypeInt {
    /// All [`TypeInt`] variants
    pub const ALL: &[Self] = &[Self::I8, Self::I16, Self::I32, Self::I64];

    /// How many bytes this [`TypeInt`] is.
    #[inline]
    pub fn to_bytes(&self) -> u32 {
        match self {
            Self::I8 => 1,
            Self::I16 => 2,
            Self::I32 => 4,
            Self::I64 => 8,
        }
    }

    /// How many bits this [`TypeInt`] is.
    #[inline]
    pub fn to_bits(&self) -> u32 {
        self.to_bytes() * 8
    }

    /// convert to `rspirv2`'s [`IdResultType`], assuming the module has the [`preamble`]
    #[inline]
    pub fn to_rspirv2(&self) -> IdResultType {
        match self {
            Self::I8 => SPIRV_I8,
            Self::I16 => SPIRV_I16,
            Self::I32 => SPIRV_I32,
            Self::I64 => SPIRV_I64,
        }
    }
}

/// The global spirv preamble to define all the types
pub fn preamble() -> InstVec<CoreInstSet> {
    let mut vec = InstVec::new();
    push_preamble(&mut vec);
    vec
}

/// Like [`preamble`] but pushes it into an existing vec
pub fn push_preamble(vec: &mut InstVec<CoreInstSet>) {
    vec.push(OpTypeVoid {
        id_result: SPIRV_VOID.0,
    });
    vec.push(OpTypeBool {
        id_result: SPIRV_BOOL.0,
    });
    for i in TypeInt::ALL {
        vec.push(OpTypeInt {
            id_result: i.to_rspirv2().0,
            width: LiteralInteger::new(i.to_bits()),
            signedness: LiteralInteger::new(0),
        });
    }
}

/// The main DisContext for disasm
///
/// Could probably be cached in a OnceLock, but that's `std`.
pub fn dis_context() -> DisContext {
    let mut context = DisContext::no_context(DisOptions::simple());
    context.add_context(&preamble());
    context
}
