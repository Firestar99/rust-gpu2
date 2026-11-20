use cranelift_codegen::ir;
use cranelift_codegen::isa::CallConv;

/// Translate a [`waffle::Type`] to a cranelift [`ir::Type`]
pub fn lower_type(ty: waffle::Type) -> anyhow::Result<ir::Type> {
    Ok(match ty {
        waffle::Type::I32 => ir::types::I32,
        waffle::Type::I64 => ir::types::I64,
        waffle::Type::F32 => ir::types::F32,
        waffle::Type::F64 => ir::types::F64,
        _ => return Err(anyhow::anyhow!("Can't translate type {ty}")),
    })
}

/// Translate a [`waffle::Signature`] reference to a cranelift [`ir::Signature`]
pub fn lower_sig(
    module: &waffle::Module,
    sig: waffle::Signature,
    call_conv: CallConv,
) -> anyhow::Result<ir::Signature> {
    let sig = &module.signatures[sig];
    let trans_abi = |tys: &[waffle::Type]| {
        tys.iter()
            .copied()
            .map(|ty| {
                Ok(ir::AbiParam {
                    value_type: lower_type(ty)?,
                    purpose: ir::ArgumentPurpose::Normal,
                    extension: ir::ArgumentExtension::None,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()
    };
    Ok(ir::Signature {
        params: trans_abi(&sig.params)?,
        returns: trans_abi(&sig.returns)?,
        call_conv,
    })
}
