use anyhow::Context;
use cranelift_codegen::entity::{EntityList, ListPool};
use cranelift_codegen::ir;
use cranelift_codegen::ir::{
    ArgumentExtension, ArgumentPurpose, BlockArg, BlockCall, InstBuilder, JumpTableData, TrapCode,
    UserFuncName,
};
use cranelift_codegen::isa::CallConv;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use smallvec::SmallVec;
use waffle::entity::EntityRef;
use waffle::{BlockTarget, Terminator, ValueDef};

mod compute;
pub use compute::*;

/// A [`SmallVec`] of [`ir::Value`]
pub type SmallValueVec = SmallVec<[ir::Value; 4]>;

#[cfg(target_pointer_width = "64")]
const _: () = {
    // 4 `ir::Values` is free
    assert!(size_of::<SmallVec<[ir::Value; 4]>>() == size_of::<SmallVec<[ir::Value; 1]>>())
};

pub trait Translator {
    fn lower(
        &self,
        builder: &mut FunctionBuilder,
        op: waffle::Operator,
        args: &[ir::Value],
    ) -> anyhow::Result<Option<SmallValueVec>>;
}

/// Options for waffle to cranelift translation
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct TranslatorOptions {
    /// The namespace id to use in [`UserExternalName::new`].
    pub clif_namespace: u32,
    /// The [`CallConv`] to use for translated functions
    pub call_conv: CallConv,
    /// [`TrapCode`] for reaching an unreachable
    pub unreachable_trap_code: TrapCode,
}

impl Default for TranslatorOptions {
    fn default() -> Self {
        Self {
            clif_namespace: 0,
            call_conv: CallConv::Fast,
            unreachable_trap_code: TrapCode::user(1).unwrap(),
        }
    }
}

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

pub fn lower_func(
    module: &waffle::Module,
    func: waffle::Func,
    opts: &TranslatorOptions,
) -> anyhow::Result<ir::Function> {
    let func = &module.funcs[func];
    let sig = &module.signatures[func.sig()];
    let trans_abi = |tys: &[waffle::Type]| {
        tys.iter()
            .copied()
            .map(|ty| {
                Ok(ir::AbiParam {
                    value_type: lower_type(ty)?,
                    purpose: ArgumentPurpose::Normal,
                    extension: ArgumentExtension::None,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()
    };
    Ok(ir::Function::with_name_signature(
        UserFuncName::user(opts.clif_namespace, 0),
        ir::Signature {
            params: trans_abi(&sig.params)?,
            returns: trans_abi(&sig.returns)?,
            call_conv: CallConv::Fast,
        },
    ))
}

pub fn lower_body(
    func_bctx: &mut FunctionBuilderContext,
    module: &waffle::Module,
    func: waffle::Func,
    translator: impl Translator,
    opts: &TranslatorOptions,
) -> anyhow::Result<()> {
    let func_decl = &module.funcs[func];
    let body = func_decl
        .body()
        .with_context(|| format!("Function must have a body: {:?}", func_decl))?;

    let mut func = lower_func(module, func, opts)?;
    let mut bdr = FunctionBuilder::new(&mut func, func_bctx);
    let b2b = body
        .blocks
        .iter()
        .map(|_| bdr.create_block())
        .collect::<Vec<_>>();

    let mut v2v = V2V::new(body.values.len());
    let mut args_c = CachedValueVec::default();
    for (block_i, block) in body.blocks.entries() {
        let block_c = b2b[block_i.index()];
        bdr.switch_to_block(block_c);
        for &(wty, value_w) in &block.params {
            let value_c = bdr.append_block_param(block_c, lower_type(wty)?);
            v2v.set_single(value_w, value_c);
        }

        for &result_w in &block.insts {
            let inst_w = &body.values[result_w];
            match inst_w {
                // block params are added above
                ValueDef::BlockParam(_, _, _) => (),
                ValueDef::Operator(waffle::Operator::Nop, _, _) => (),
                &ValueDef::Operator(op, args_w, _) => {
                    let result = translator
                        .lower(&mut bdr, op, args_c.setup(&v2v, &body.arg_pool[args_w]))?
                        .with_context(|| format!("Operation {op} couldn't be lowered"))?;
                    v2v.set(result_w, result);
                }
                &ValueDef::PickOutput(value_w, i, _) => {
                    v2v.set_single(result_w, v2v.get(value_w)[i as usize])
                }
                &ValueDef::Alias(value_w) => {
                    v2v.alias(result_w, value_w);
                }
                ValueDef::Placeholder(_) | ValueDef::None => {
                    panic!("Value {result_w} has impossible ValueDef {inst_w:?}");
                }
            }
        }

        match &block.terminator {
            Terminator::Br { target } => {
                bdr.ins().jump(
                    b2b[target.block.index()],
                    v2v.block_args(&target.args).iter(),
                );
            }
            Terminator::CondBr {
                cond,
                if_true,
                if_false,
            } => {
                bdr.ins().brif(
                    v2v.get_single(*cond),
                    b2b[if_true.block.index()],
                    v2v.block_args(&if_true.args).iter(),
                    b2b[if_false.block.index()],
                    v2v.block_args(&if_false.args).iter(),
                );
            }
            Terminator::Select {
                value,
                default,
                targets,
            } => {
                let mut block_call = |target: &BlockTarget| {
                    BlockCall::new(
                        b2b[target.block.index()],
                        v2v.block_args_iter(&target.args),
                        &mut bdr.func.dfg.value_lists,
                    )
                };
                let table = JumpTableData::new(
                    block_call(&default),
                    &targets.iter().map(block_call).collect::<SmallVec<[_; 8]>>(),
                );
                let table = bdr.create_jump_table(table);
                bdr.ins().br_table(v2v.get_single(*value), table);
            }
            Terminator::Return { values } => {
                bdr.ins().return_(
                    &values
                        .iter()
                        .map(|v| v2v.get_single(*v))
                        .collect::<SmallVec<[_; 8]>>(),
                );
            }
            Terminator::Unreachable => {
                bdr.ins().trap(opts.unreachable_trap_code);
            }
            Terminator::None => panic!("unterminated block {block_i}"),
        };
    }
    bdr.seal_all_blocks();

    Ok(())
}

/// [`waffle::Value`] to cranelift [`ir::Value`] SecondaryMap that can be filled in on the go.
#[derive(Clone, Debug)]
pub struct V2V {
    pub map: Vec<EntityList<ir::Value>>,
    pub pool: ListPool<ir::Value>,
}

impl V2V {
    pub fn new(capacity: usize) -> Self {
        Self {
            map: vec![EntityList::default(); capacity],
            pool: ListPool::default(),
        }
    }

    pub fn set(&mut self, key: waffle::Value, values: impl IntoIterator<Item = ir::Value>) {
        self.map[key.index()] = EntityList::from_iter(values, &mut self.pool);
    }

    pub fn set_single(&mut self, key: waffle::Value, value: ir::Value) {
        self.set(key, [value])
    }

    pub fn get(&self, key: waffle::Value) -> &[ir::Value] {
        self.map[key.index()].as_slice(&self.pool)
    }

    pub fn get_single(&self, key: waffle::Value) -> ir::Value {
        let slice = self.get(key);
        assert_eq!(slice.len(), 1, "Expected single value");
        slice[0]
    }

    pub fn alias(&mut self, key: waffle::Value, to: waffle::Value) {
        self.map[to.index()] = self.map[key.index()];
    }

    pub fn block_args_iter<'a>(
        &self,
        keys: impl IntoIterator<Item = &'a waffle::Value>,
    ) -> impl Iterator<Item = BlockArg> {
        keys.into_iter()
            .map(|v| BlockArg::Value(self.get_single(*v)))
    }

    pub fn block_args<'a>(
        &self,
        keys: impl IntoIterator<Item = &'a waffle::Value>,
    ) -> SmallVec<[BlockArg; 8]> {
        self.block_args_iter(keys).collect()
    }
}

#[derive(Default)]
pub struct CachedValueVec(SmallValueVec);

impl CachedValueVec {
    pub fn setup(&mut self, v2v: &V2V, waffle_values: &[waffle::Value]) -> &[ir::Value] {
        self.0.clear();
        self.0
            .extend(waffle_values.into_iter().map(|v| v2v.get_single(*v)));
        self.0.as_slice()
    }
}
