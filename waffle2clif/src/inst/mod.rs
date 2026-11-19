use cranelift_codegen::ir;
use cranelift_frontend::FunctionBuilder;
use smallvec::SmallVec;

mod compute;
pub use compute::*;
mod call;
pub use call::*;

/// A [`SmallVec`] of [`ir::Value`]
pub type SmallValueVec = SmallVec<[ir::Value; 4]>;

#[cfg(target_pointer_width = "64")]
const _: () = {
    // 4 `ir::Values` is free
    assert!(size_of::<SmallVec<[ir::Value; 4]>>() == size_of::<SmallVec<[ir::Value; 1]>>())
};

/// A semantic lowerer from waffle's SSA-shaped wasm to cranelift's clif
pub trait Waffle2Clif: Sized {
    fn lower(
        &self,
        builder: &mut FunctionBuilder,
        op: waffle::Operator,
        args: &[ir::Value],
    ) -> anyhow::Result<Option<SmallValueVec>>;

    fn chain<B: Waffle2Clif>(&self, b: B) -> ChainedWaffle2Clif<&Self, B> {
        ChainedWaffle2Clif(self, b)
    }
}

impl<T: Waffle2Clif> Waffle2Clif for &T {
    fn lower(
        &self,
        builder: &mut FunctionBuilder,
        op: waffle::Operator,
        args: &[ir::Value],
    ) -> anyhow::Result<Option<SmallValueVec>> {
        T::lower(self, builder, op, args)
    }
}

pub struct ChainedWaffle2Clif<A: Waffle2Clif, B: Waffle2Clif>(pub A, pub B);

impl<A: Waffle2Clif, B: Waffle2Clif> Waffle2Clif for ChainedWaffle2Clif<A, B> {
    fn lower(
        &self,
        builder: &mut FunctionBuilder,
        op: waffle::Operator,
        args: &[ir::Value],
    ) -> anyhow::Result<Option<SmallValueVec>> {
        match self.0.lower(&mut *builder, op, args) {
            Ok(None) => self.1.lower(builder, op, args),
            e => e,
        }
    }
}
