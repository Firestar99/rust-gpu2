use crate::{SmallValueVec, Translator};
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::immediates::{Ieee32, Ieee64};
use cranelift_codegen::ir::types::*;
use cranelift_codegen::ir::{InstBuilder, MemFlagsData, Value};
use cranelift_frontend::FunctionBuilder;
use waffle::Operator;

pub struct Compute;

impl Translator for Compute {
    fn lower(
        &self,
        bdr: &mut FunctionBuilder,
        op: Operator,
        args: &[Value],
    ) -> Result<Option<SmallValueVec>, ()> {
        use waffle::Operator::*;
        let arg = |i: usize| args.get(i).copied().ok_or(());

        // IntCC
        let compare = match op {
            I32Eq | I64Eq => Some(IntCC::Equal),
            I32Ne | I64Ne => Some(IntCC::NotEqual),
            I32LtU | I64LtU => Some(IntCC::UnsignedLessThan),
            I32GtU | I64GtU => Some(IntCC::UnsignedGreaterThan),
            I32LeU | I64LeU => Some(IntCC::UnsignedLessThanOrEqual),
            I32GeU | I64GeU => Some(IntCC::UnsignedGreaterThanOrEqual),
            I32LtS | I64LtS => Some(IntCC::SignedLessThan),
            I32GtS | I64GtS => Some(IntCC::SignedGreaterThan),
            I32LeS | I64LeS => Some(IntCC::SignedLessThanOrEqual),
            I32GeS | I64GeS => Some(IntCC::SignedGreaterThanOrEqual),
            _ => None,
        };
        if let Some(intcc) = compare {
            let cmp = bdr.ins().icmp(intcc, arg(0)?, arg(1)?);
            let result = bdr.ins().uextend(I32, cmp);
            return Ok(Some(SmallValueVec::from_iter([result])));
        }

        // FloatCC
        let compare = match op {
            F32Eq | F64Eq => Some(FloatCC::Equal),
            F32Ne | F64Ne => Some(FloatCC::NotEqual),
            F32Lt | F64Lt => Some(FloatCC::LessThan),
            F32Gt | F64Gt => Some(FloatCC::GreaterThan),
            F32Le | F64Le => Some(FloatCC::LessThanOrEqual),
            F32Ge | F64Ge => Some(FloatCC::GreaterThanOrEqual),
            _ => None,
        };
        if let Some(floatcc) = compare {
            let cmp = bdr.ins().fcmp(floatcc, arg(0)?, arg(1)?);
            let result = bdr.ins().uextend(I32, cmp);
            return Ok(Some(SmallValueVec::from_iter([result])));
        }

        let result = match op {
            // const
            I32Const { value } => bdr.ins().iconst(I32, value as u64 as i64),
            I64Const { value } => bdr.ins().iconst(I64, value as i64),
            F32Const { value } => bdr.ins().f32const(Ieee32::with_bits(value)),
            F64Const { value } => bdr.ins().f64const(Ieee64::with_bits(value)),

            // special
            I32Eqz | I64Eqz => bdr.ins().icmp_imm_u(IntCC::Equal, arg(0)?, 0),

            // int
            I32Clz | I64Clz => bdr.ins().clz(arg(0)?),
            I32Ctz | I64Ctz => bdr.ins().ctz(arg(0)?),
            I32Popcnt | I64Popcnt => bdr.ins().popcnt(arg(0)?),
            I32Add | I64Add => bdr.ins().iadd(arg(0)?, arg(1)?),
            I32Sub | I64Sub => bdr.ins().isub(arg(0)?, arg(1)?),
            I32Mul | I64Mul => bdr.ins().imul(arg(0)?, arg(1)?),
            I32DivU | I64DivU => bdr.ins().udiv(arg(0)?, arg(1)?),
            I32DivS | I64DivS => bdr.ins().sdiv(arg(0)?, arg(1)?),
            I32RemU | I64RemU => bdr.ins().urem(arg(0)?, arg(1)?),
            I32RemS | I64RemS => bdr.ins().srem(arg(0)?, arg(1)?),
            I32And | I64And => bdr.ins().band(arg(0)?, arg(1)?),
            I32Or | I64Or => bdr.ins().bor(arg(0)?, arg(1)?),
            I32Xor | I64Xor => bdr.ins().bxor(arg(0)?, arg(1)?),
            I32Shl | I64Shl => bdr.ins().ishl(arg(0)?, arg(1)?),
            I32ShrU | I64ShrU => bdr.ins().ushr(arg(0)?, arg(1)?),
            I32ShrS | I64ShrS => bdr.ins().sshr(arg(0)?, arg(1)?),
            I32Rotl | I64Rotl => bdr.ins().rotl(arg(0)?, arg(1)?),
            I32Rotr | I64Rotr => bdr.ins().rotr(arg(0)?, arg(1)?),

            // float
            F32Abs | F64Abs => bdr.ins().fabs(arg(0)?),
            F32Neg | F64Neg => bdr.ins().fneg(arg(0)?),
            F32Ceil | F64Ceil => bdr.ins().ceil(arg(0)?),
            F32Floor | F64Floor => bdr.ins().floor(arg(0)?),
            F32Trunc | F64Trunc => bdr.ins().trunc(arg(0)?),
            F32Nearest | F64Nearest => bdr.ins().nearest(arg(0)?),
            F32Sqrt | F64Sqrt => bdr.ins().sqrt(arg(0)?),
            F32Add | F64Add => bdr.ins().fadd(arg(0)?, arg(1)?),
            F32Sub | F64Sub => bdr.ins().fsub(arg(0)?, arg(1)?),
            F32Mul | F64Mul => bdr.ins().fmul(arg(0)?, arg(1)?),
            F32Div | F64Div => bdr.ins().fdiv(arg(0)?, arg(1)?),
            F32Min | F64Min => bdr.ins().fmin(arg(0)?, arg(1)?),
            F32Max | F64Max => bdr.ins().fmax(arg(0)?, arg(1)?),
            F32Copysign | F64Copysign => bdr.ins().fcopysign(arg(0)?, arg(1)?),

            // int type extension
            I32WrapI64 => bdr.ins().ireduce(I32, arg(0)?),
            I64ExtendI32U => bdr.ins().uextend(I32, arg(0)?),
            I64ExtendI32S => bdr.ins().sextend(I32, arg(0)?),

            // f2i
            I32TruncF32U | I32TruncF64U => bdr.ins().fcvt_to_uint(I32, arg(0)?),
            I32TruncF32S | I32TruncF64S => bdr.ins().fcvt_to_sint(I32, arg(0)?),
            I64TruncF32U | I64TruncF64U => bdr.ins().fcvt_to_uint(I64, arg(0)?),
            I64TruncF32S | I64TruncF64S => bdr.ins().fcvt_to_sint(I64, arg(0)?),
            I32TruncSatF32U | I32TruncSatF64U => bdr.ins().fcvt_to_uint_sat(I32, arg(0)?),
            I32TruncSatF32S | I32TruncSatF64S => bdr.ins().fcvt_to_sint_sat(I32, arg(0)?),
            I64TruncSatF32U | I64TruncSatF64U => bdr.ins().fcvt_to_uint_sat(I64, arg(0)?),
            I64TruncSatF32S | I64TruncSatF64S => bdr.ins().fcvt_to_sint_sat(I64, arg(0)?),

            // i2f
            F32ConvertI32U | F32ConvertI64U => bdr.ins().fcvt_from_uint(F32, arg(0)?),
            F32ConvertI32S | F32ConvertI64S => bdr.ins().fcvt_from_sint(F32, arg(0)?),
            F64ConvertI32U | F64ConvertI64U => bdr.ins().fcvt_from_uint(F64, arg(0)?),
            F64ConvertI32S | F64ConvertI64S => bdr.ins().fcvt_from_sint(F64, arg(0)?),

            // f2f
            F32DemoteF64 => bdr.ins().fdemote(F32, arg(0)?),
            F64PromoteF32 => bdr.ins().fpromote(F64, arg(0)?),

            // sextend
            I32Extend8S => bdr.ins().sextend(I32, bdr.ins().ireduce(I8, arg(0)?)),
            I32Extend16S => bdr.ins().sextend(I32, bdr.ins().ireduce(I16, arg(0)?)),
            I64Extend8S => bdr.ins().sextend(I64, bdr.ins().ireduce(I8, arg(0)?)),
            I64Extend16S => bdr.ins().sextend(I64, bdr.ins().ireduce(I16, arg(0)?)),
            I64Extend32S => bdr.ins().sextend(I64, bdr.ins().ireduce(I32, arg(0)?)),

            // bitcast
            F32ReinterpretI32 => bdr.ins().bitcast(F32, MemFlagsData::new(), arg(0)?),
            F64ReinterpretI64 => bdr.ins().bitcast(F64, MemFlagsData::new(), arg(0)?),
            I32ReinterpretF32 => bdr.ins().bitcast(I32, MemFlagsData::new(), arg(0)?),
            I64ReinterpretF64 => bdr.ins().bitcast(I64, MemFlagsData::new(), arg(0)?),
            _ => return Ok(None),
        };
        Ok(Some(SmallValueVec::from_iter([result])))
    }
}
