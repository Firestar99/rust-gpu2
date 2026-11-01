//! An interface to abstract away the unnecessary details for SSA-shaped IRs
//!
//! This implies:
//! * virtual registery only: SSA values are modeled as infinitely many virtual registers that don't need to be reused.
//! * no regalloc: With SSA values being virtual registers, there's no need for physical registers or regalloc.
//! * no stack spilling: values are passed and retuned as SSA values and never spilled to a stack.
//! * reachable only with explicit stack slots: [`Callee::gen_prologue`] generates a stack overflow check, calling
//! this function
//!
//! ### Stack is unsupported
//!
//! This implementation does not support a stack. Since all values are passed and returned in registers, this isn't a
//! problem for ABI. However, clif containing **explict StackSlots** is unsupported and will likely result in calling
//! an unreachable.
//!
//! Waffle2clif emits clean SSA with bbargs and doesn't make use of StackSlots, so there hasn't been any need to
//! add them. Other `cranelift-frontend` users may make use of explicit StackSlots, such as `rustc_codegen_cranelift`.
//! If you do encounter this problem, you've got 3 options:
//! * make the frontend not emit any explicit stack slots, `cranelift-frontend`'s registers should all compile down to
//!   nice SSA without explicit StackSlots.
//! * add support for explicit StackSlots here
//! * add a "mem2reg" pass for explicit StackSlots in cranelift, converting any use of them into clean SSA.
//!

use crate::binemit::{Addend, CodeOffset, Reloc};
use crate::ir::{AbiParam, ArgumentExtension, ArgumentPurpose, Signature, Type};
use crate::isa::{CallConv, FunctionAlignment};
use crate::machinst::isle::WritableReg;
use crate::machinst::{
    ABIArg, ABIArgLocation, ABIArgSlot, ABIArgSlotVec, ABIMachineSpec, ArgPair, ArgsAccumulator,
    ArgsOrRets, CallType, FunctionCalls, IsaFlags, MachInstLabelUse, MachTerminator,
    OperandVisitor, RetPair, SmallInstVec, StackAMode,
};
use crate::settings::Flags;
use crate::{
    CodegenResult, FrameLayout, MachInst, MachInstEmit, MachLabel, RealReg, Reg, RegClass, Writable,
};
use alloc::vec::Vec;
use core::fmt::Debug;
use core::marker::PhantomData;
use regalloc2::{MachineEnv, PReg, PRegSet, VReg};
use smallvec::SmallVec;

pub const SSA_REG_CLASS: RegClass = RegClass::Int;

pub trait SsaInst: Clone + Debug {
    /// The ISA flags type.
    type F: IsaFlags;

    /// Returns the number of bits in a word, that is 32/64 for 32/64-bit architecture.
    fn word_bits() -> u32;

    /// Generate an "args" pseudo-instruction to capture input args in
    /// registers.
    fn gen_args(args: Vec<ArgPair>) -> Self;

    /// Generate a "rets" pseudo-instruction that moves vregs to return
    /// registers.
    fn gen_rets(rets: Vec<RetPair>) -> Self;

    /// Generate a return instruction.
    fn gen_return(isa_flags: &Self::F) -> SmallInstVec<Self>;

    /// Generate a NOP. The `preferred_size` parameter allows the caller to
    /// request a NOP of that size, or as close to it as possible. The machine
    /// backend may return a NOP whose binary encoding is smaller than the
    /// preferred size, but must not return a NOP that is larger. However,
    /// the instruction must have a nonzero size if preferred_size is nonzero.
    ///
    /// SSA: unused, only used for function alignment, which is disabled
    fn gen_nop(preferred_size: usize) -> Self;

    /// The various kinds of NOP, with size, sorted in ascending-size
    /// order.
    ///
    /// SSA: only used within wasmtime for debugging
    fn gen_nop_units() -> Vec<Vec<u8>>;

    /// Return the registers referenced by this machine instruction along with
    /// the modes of reference (use, def, modify).
    fn get_operands(&mut self, collector: &mut impl OperandVisitor);

    /// Is this a terminator (branch or ret)? If so, return its type
    /// (ret/uncond/cond) and target if applicable.
    fn is_term(&self) -> MachTerminator;

    /// Is this an unconditional trap?
    fn is_trap(&self) -> bool;

    /// Is this an "args" pseudoinst?
    fn is_args(&self) -> bool;

    /// Classify the type of call instruction this is.
    ///
    /// This enables more granular function type analysis and optimization.
    /// Returns `CallType::None` for non-call instructions, `CallType::Regular`
    /// for normal calls that return to the caller, and `CallType::TailCall`
    /// for tail calls that don't return to the caller.
    fn call_type(&self) -> CallType;

    /// Does this instruction access memory?
    fn is_mem_access(&self) -> bool;

    /// Is this a safepoint?
    fn is_safepoint(&self) -> bool;
}

impl<I: SsaInst + MachInstEmit> MachInst for I {
    type ABIMachineSpec = SsaAbi<I>;

    fn get_operands(&mut self, collector: &mut impl OperandVisitor) {
        <I as SsaInst>::get_operands(self, collector)
    }

    /// SSA has no physical registers
    fn is_move(&self) -> Option<(Writable<Reg>, Reg)> {
        None
    }

    fn is_term(&self) -> MachTerminator {
        <I as SsaInst>::is_term(self)
    }

    fn is_trap(&self) -> bool {
        <I as SsaInst>::is_trap(self)
    }

    fn is_args(&self) -> bool {
        <I as SsaInst>::is_args(self)
    }

    fn call_type(&self) -> CallType {
        <I as SsaInst>::call_type(self)
    }

    /// SSA has no physical registers
    fn is_included_in_clobbers(&self) -> bool {
        false
    }

    fn is_mem_access(&self) -> bool {
        <I as SsaInst>::is_mem_access(self)
    }

    /// unreachable: no regalloc
    fn gen_move(_to_reg: Writable<Reg>, _from_reg: Reg, _ty: Type) -> Self {
        unreachable!()
    }

    /// SSA has no physical registers
    fn gen_dummy_use(_reg: Reg) -> Self {
        <I as SsaInst>::gen_nop(0)
    }

    fn rc_for_type(ty: &Type) -> CodegenResult<(&[RegClass], &[Type])> {
        Ok((&[SSA_REG_CLASS], core::slice::from_ref(ty)))
    }

    /// unreachable: no stack spilling
    fn canonical_type_for_rc(_rc: RegClass) -> Type {
        unreachable!()
    }

    /// bug oof
    fn gen_jump(_target: MachLabel) -> Self {
        unreachable!(
            "gen_jump() doesn't function on SSA IRs, since we need to explicitly pass all args. Should only be\
         reachable via emitting a critical edge in `BlockLoweringOrder`, which needs to be fixed up for SSA."
        )
    }

    fn gen_nop(preferred_size: usize) -> Self {
        <I as SsaInst>::gen_nop(preferred_size)
    }

    fn gen_nop_units() -> Vec<Vec<u8>> {
        <I as SsaInst>::gen_nop_units()
    }

    /// SSA has no jump islands
    fn worst_case_size() -> CodeOffset {
        0
    }

    /// SSA has no jump islands
    fn worst_case_island_growth() -> CodeOffset {
        0
    }

    fn is_safepoint(&self) -> bool {
        <I as SsaInst>::is_safepoint(self)
    }

    fn function_alignment() -> FunctionAlignment {
        FunctionAlignment {
            preferred: 1,
            minimum: 1,
        }
    }

    type LabelUse = SsaLabelUse;
    const TRAP_OPCODE: &'static [u8] = &[];
}

#[derive(Copy, Clone, Debug, Default)]
pub struct SsaAbi<I: SsaInst + MachInstEmit> {
    _phantom: PhantomData<I>,
}

impl<I: SsaInst + MachInstEmit> ABIMachineSpec for SsaAbi<I> {
    type I = I;
    type F = I::F;
    const STACK_ARG_RET_SIZE_LIMIT: u32 = 0;

    fn word_bits() -> u32 {
        <I as SsaInst>::word_bits()
    }

    fn stack_align(_call_conv: CallConv) -> u32 {
        1
    }

    fn compute_arg_locs(
        _call_conv: CallConv,
        _flags: &Flags,
        params: &[AbiParam],
        _args_or_rets: ArgsOrRets,
        add_ret_area_ptr: bool,
        mut args: ArgsAccumulator,
    ) -> CodegenResult<(u32, Option<usize>)> {
        // unreachable:
        // * first this function is called with `ArgsOrRets::Rets`, which always passes `add_ret_area_ptr: false`
        // * then is called with `ArgsOrRets::Args`, which has `add_ret_area_ptr: true` only if rets call returned
        //   `(x > 0, ...)`
        // * we always return `Ok(0, ...)`, so the second case never happens
        assert_eq!(add_ret_area_ptr, false);

        for (i, param) in params.iter().enumerate() {
            args.push(ABIArg::Slots {
                slots: ABIArgSlotVec::from_buf([ABIArgSlot::Reg {
                    // the RealReg `i` is mocked and doesn't do anything meaningfully
                    reg: RealReg::from(PReg::new(i, SSA_REG_CLASS)),
                    ty: param.value_type,
                    // not sure if we should pass the arg extension or force `ArgumentExtension::None`
                    extension: param.extension,
                }]),
                purpose: ArgumentPurpose::Normal,
            })
        }
        Ok((0, None))
    }

    /// unreachable: no stack spilling
    fn gen_load_stack(_mem: StackAMode, _into_reg: Writable<Reg>, _ty: Type) -> Self::I {
        unreachable!()
    }

    /// unreachable: no stack spilling
    fn gen_store_stack(_mem: StackAMode, _from_reg: Reg, _ty: Type) -> Self::I {
        unreachable!()
    }

    /// unreachable: unused in the general code path, only within aarch64 abi
    fn gen_move(_to_reg: Writable<Reg>, _from_reg: Reg, _ty: Type) -> Self::I {
        unreachable!()
    }

    /// unreachable: as long as [`Self::get_ext_mode`] returns [`ArgumentExtension::None`]
    fn gen_extend(
        _to_reg: Writable<Reg>,
        _from_reg: Reg,
        _is_signed: bool,
        _from_bits: u8,
        _to_bits: u8,
    ) -> Self::I {
        unreachable!()
    }

    fn gen_args(args: Vec<ArgPair>) -> Self::I {
        <I as SsaInst>::gen_args(args)
    }

    fn gen_rets(rets: Vec<RetPair>) -> Self::I {
        <I as SsaInst>::gen_rets(rets)
    }

    /// unsupported stack: reachable only with explicit stack slots as [`Callee::gen_prologue`] generates a stack
    /// overflow check, calling this function
    ///
    /// [`Callee::gen_prologue`]: crate::machinst::Callee::gen_prologue
    fn gen_add_imm(
        _call_conv: CallConv,
        _into_reg: Writable<Reg>,
        _from_reg: Reg,
        _imm: u32,
    ) -> SmallInstVec<Self::I> {
        unreachable_stack()
    }

    /// unsupported stack: reachable only with explicit stack slots as [`Callee::gen_prologue`] generates a stack
    /// overflow check, calling this function
    ///
    /// [`Callee::gen_prologue`]: crate::machinst::Callee::gen_prologue
    fn gen_stack_lower_bound_trap(_limit_reg: Reg) -> SmallInstVec<Self::I> {
        unreachable_stack()
    }

    /// unsupported stack: reachable only with explicit stack slot API:
    /// * [`Callee::sized_stackslot_addr`]
    /// * [`Callee::dynamic_stackslot_addr`]
    /// * unreachable: [`SigData::stack_ret_arg`] that returns `Some`, causing ABI to setup a stack return area,
    ///   which we disable in [`Self::compute_arg_locs`]
    ///
    /// [`Callee::sized_stackslot_addr`]: crate::machinst::Callee::sized_stackslot_addr
    /// [`Callee::dynamic_stackslot_addr`]: crate::machinst::Callee::dynamic_stackslot_addr
    /// [`SigData::stack_ret_arg`]: crate::machinst::SigData::stack_ret_arg
    fn gen_get_stack_addr(_mem: StackAMode, _into_reg: Writable<Reg>) -> Self::I {
        unreachable_stack()
    }

    /// unsupported stack: reachable only if the function declares a [`FunctionStencil::stack_limit`]
    fn get_stacklimit_reg(_call_conv: CallConv) -> Reg {
        unreachable_stack_limit()
    }

    /// unsupported stack: reachable only if the function declares a [`FunctionStencil::stack_limit`]
    fn gen_load_base_offset(
        _into_reg: Writable<Reg>,
        _base: Reg,
        _offset: i32,
        _ty: Type,
    ) -> Self::I {
        unreachable_stack_limit()
    }

    /// unreachable: no stack spilling
    fn gen_store_base_offset(_base: Reg, _offset: i32, _from_reg: Reg, _ty: Type) -> Self::I {
        unreachable!()
    }

    /// unreachable: unused (outside of ISA-specific use cases)
    fn gen_sp_reg_adjust(_amount: i32) -> SmallInstVec<Self::I> {
        unreachable!()
    }

    /// mock frame layout
    fn compute_frame_layout(
        _call_conv: CallConv,
        _flags: &Flags,
        _sig: &Signature,
        _regs: &[Writable<RealReg>],
        function_calls: FunctionCalls,
        incoming_args_size: u32,
        tail_args_size: u32,
        stackslots_size: u32,
        fixed_frame_storage_size: u32,
        outgoing_args_size: u32,
    ) -> FrameLayout {
        let layout = FrameLayout {
            word_bytes: Self::word_bytes(),
            incoming_args_size,
            tail_args_size,
            setup_area_size: 0,
            clobber_size: 0,
            fixed_frame_storage_size,
            stackslots_size,
            outgoing_args_size,
            clobbered_callee_saves: vec![],
            function_calls,
        };
        assert_eq!(
            layout.active_size(),
            0,
            "SSA doesn't support StackSlots yet"
        );
        layout
    }

    /// mock noop
    fn gen_prologue_frame_setup(
        _call_conv: CallConv,
        _flags: &Flags,
        _isa_flags: &Self::F,
        _frame_layout: &FrameLayout,
    ) -> SmallInstVec<Self::I> {
        SmallInstVec::new()
    }

    /// mock noop
    fn gen_epilogue_frame_restore(
        _call_conv: CallConv,
        _flags: &Flags,
        _isa_flags: &Self::F,
        _frame_layout: &FrameLayout,
    ) -> SmallInstVec<Self::I> {
        SmallInstVec::new()
    }

    /// needs a proper impl
    fn gen_return(
        _call_conv: CallConv,
        isa_flags: &Self::F,
        _frame_layout: &FrameLayout,
    ) -> SmallInstVec<Self::I> {
        <I as SsaInst>::gen_return(isa_flags)
    }

    /// unsupported stack
    fn gen_probestack(_insts: &mut SmallInstVec<Self::I>, _frame_size: u32) {
        unreachable_stack()
    }

    /// unsupported stack
    fn gen_inline_probestack(
        _insts: &mut SmallInstVec<Self::I>,
        _call_conv: CallConv,
        _frame_size: u32,
        _guard_size: u32,
    ) {
        unreachable_stack()
    }

    /// always called, noop
    fn gen_clobber_save(
        _call_conv: CallConv,
        _flags: &Flags,
        _frame_layout: &FrameLayout,
    ) -> SmallVec<[Self::I; 16]> {
        SmallVec::new()
    }

    /// always called, noop
    fn gen_clobber_restore(
        _call_conv: CallConv,
        _flags: &Flags,
        _frame_layout: &FrameLayout,
    ) -> SmallVec<[Self::I; 16]> {
        SmallVec::new()
    }

    /// unreachable: requires [`ABIArg::StructArg`] but [`Self::compute_arg_locs`] always returns slots
    fn gen_memcpy<F: FnMut(Type) -> Writable<Reg>>(
        _call_conv: CallConv,
        _dst: Reg,
        _src: Reg,
        _size: usize,
        _alloc_tmp: F,
    ) -> SmallVec<[Self::I; 8]> {
        unreachable!()
    }

    /// unreachable: used in reg alloc
    fn get_number_of_spillslots_for_value(
        _rc: RegClass,
        _target_vector_bytes: u32,
        _isa_flags: &Self::F,
    ) -> u32 {
        unreachable!()
    }

    /// **reachable but unused**: used in reg alloc
    fn get_machine_env(_flags: &Flags, _call_conv: CallConv) -> &MachineEnv {
        const {
            &MachineEnv {
                preferred_regs_by_class: [PRegSet::empty(), PRegSet::empty(), PRegSet::empty()],
                non_preferred_regs_by_class: [PRegSet::empty(), PRegSet::empty(), PRegSet::empty()],
                scratch_by_class: [None, None, None],
                fixed_stack_slots: vec![],
            }
        }
    }

    /// no phy registers means no clobbers
    fn get_regs_clobbered_by_call(_call_conv_of_callee: CallConv, _is_exception: bool) -> PRegSet {
        PRegSet::empty()
    }

    /// SSA IRs require explicit zero/sign extension, so always returns [`ArgumentExtension::None`].
    ///
    /// This requirement also makes several other functions unreachable.
    fn get_ext_mode(
        _call_conv: CallConv,
        _specified: ArgumentExtension,
        _location: ABIArgLocation,
    ) -> ArgumentExtension {
        ArgumentExtension::None
    }

    /// **reachable but unused**: we must never encounter a [`crate::machinst::RetLocation::Stack`], ensured by
    /// [`Self::compute_arg_locs`], so we can return garbage
    fn retval_temp_reg(_call_conv_of_callee: CallConv) -> Writable<Reg> {
        // This is a garbage register that'll never be used
        WritableReg::from_reg(Reg::from_virtual_reg(VReg::new(0xDEADBEEF, SSA_REG_CLASS)))
    }
}

/// A fake Label for SSA-shaped IRs that instead of PC-based jumps rely on symbolic labels to jump to.
///
/// These symbolic labels are emitted directly at codegen time and don't need to be patched up in any way,
/// so `impl MachInstLabelUse for SsaLabel` is just a dummy implementation with lots of unreachable functions.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SsaLabelUse;

impl MachInstLabelUse for SsaLabelUse {
    const ALIGN: CodeOffset = 1;

    fn max_pos_range(self) -> CodeOffset {
        u32::MAX
    }

    fn max_neg_range(self) -> CodeOffset {
        u32::MIN
    }

    fn patch_size(self) -> CodeOffset {
        0
    }

    fn patch(self, _buffer: &mut [u8], _use_offset: CodeOffset, _label_offset: CodeOffset) {
        unreachable!()
    }

    fn supports_veneer(self) -> bool {
        false
    }

    fn veneer_size(self) -> CodeOffset {
        0
    }

    fn worst_case_veneer_size() -> CodeOffset {
        0
    }

    fn generate_veneer(self, _buffer: &mut [u8], _veneer_offset: CodeOffset) -> (CodeOffset, Self) {
        unreachable!()
    }

    fn from_reloc(_reloc: Reloc, _addend: Addend) -> Option<Self> {
        None
    }
}

fn unreachable_stack_limit() -> ! {
    unreachable!("stack limit is unsupported")
}

fn unreachable_stack() -> ! {
    unreachable!("stack is not yet supported by SSA infra")
}
