extern crate alloc;

use crate::pad_to_power_of_two;
use alloc::vec;
use alloc::vec::Vec;
use columns::{Shift32Cols, COL_MAP, NUM_COLS};
use core::mem::transmute;
use valida_bus::{MachineWithGeneralBus, MachineWithPowerOfTwoBus, MachineWithRangeBus8};
use valida_cpu::MachineWithCpuChip;
use valida_machine::{instructions, Chip, Instruction, Interaction, Operands, Word};
use valida_opcodes::{DIV32, MUL32, SHL32, SHR32};
use valida_range::MachineWithRangeChip;

use p3_air::VirtualPairCol;
use p3_field::PrimeField;
use p3_matrix::dense::RowMajorMatrix;
use p3_maybe_rayon::*;

pub mod columns;
pub mod stark;

#[derive(Clone)]
pub enum Operation {
    Shl32(Word<u8>, Word<u8>, Word<u8>), // (dst, src, shift)
    Shr32(Word<u8>, Word<u8>, Word<u8>), // ''
}

#[derive(Default)]
pub struct Shift32Chip {
    pub operations: Vec<Operation>,
}

impl<F, M> Chip<M> for Shift32Chip
where
    F: PrimeField,
    M: MachineWithGeneralBus<F = F> + MachineWithRangeBus8 + MachineWithPowerOfTwoBus,
{
    fn generate_trace(&self, _machine: &M) -> RowMajorMatrix<M::F> {
        let rows = self
            .operations
            .par_iter()
            .map(|op| self.op_to_row(op))
            .collect::<Vec<_>>();

        let mut trace =
            RowMajorMatrix::new(rows.into_iter().flatten().collect::<Vec<_>>(), NUM_COLS);

        pad_to_power_of_two::<NUM_COLS, F>(&mut trace.values);

        trace
    }

    fn global_sends(&self, machine: &M) -> Vec<Interaction<M::F>> {
        // Power of two bus
        let shift = VirtualPairCol::single_main(COL_MAP.input_2[3]);
        let power_of_two = COL_MAP.power_of_two.0.map(VirtualPairCol::single_main);
        let mut fields = vec![shift];
        fields.extend(power_of_two.clone());
        let power_of_two_send = Interaction {
            fields,
            count: VirtualPairCol::one(),
            argument_index: machine.power_of_two_bus(),
        };

        // General bus (multiplication and division)
        let opcode = VirtualPairCol::new_main(
            vec![
                (COL_MAP.is_shl, M::F::from_canonical_u32(MUL32)),
                (COL_MAP.is_shr, M::F::from_canonical_u32(DIV32)),
            ],
            M::F::ZERO,
        );
        let input_1 = COL_MAP.input.0.map(VirtualPairCol::single_main);
        let output = COL_MAP.output.0.map(VirtualPairCol::single_main);
        let clk_or_zero = VirtualPairCol::constant(M::F::ZERO);
        let mut fields = vec![opcode];
        fields.extend(input_1);
        fields.extend(power_of_two);
        fields.extend(output.clone());
        fields.push(clk_or_zero);
        let general_bus_send = Interaction {
            fields,
            count: VirtualPairCol::one(),
            argument_index: machine.general_bus(),
        };

        vec![power_of_two_send, general_bus_send]
    }

    fn global_receives(&self, machine: &M) -> Vec<Interaction<M::F>> {
        let opcode = VirtualPairCol::new_main(
            vec![
                (COL_MAP.is_shl, M::F::from_canonical_u32(SHL32)),
                (COL_MAP.is_shr, M::F::from_canonical_u32(SHR32)),
            ],
            M::F::ZERO,
        );
        let input_1 = COL_MAP.input_1.0.map(VirtualPairCol::single_main);
        let input_2 = COL_MAP.input_2.0.map(VirtualPairCol::single_main);
        let output = COL_MAP.output.0.map(VirtualPairCol::single_main);

        let mut fields = vec![opcode];
        fields.extend(input_1);
        fields.extend(input_2);
        fields.extend(output);

        let is_real = VirtualPairCol::sum_main(vec![COL_MAP.is_shl, COL_MAP.is_shr]);

        let receive = Interaction {
            fields,
            count: is_real,
            argument_index: machine.general_bus(),
        };
        vec![receive]
    }
}

impl Shift32Chip {
    fn op_to_row<F>(&self, op: &Operation) -> [F; NUM_COLS]
    where
        F: PrimeField,
    {
        let mut row = [F::ZERO; NUM_COLS];
        let cols: &mut Shift32Cols<F> = unsafe { transmute(&mut row) };

        match op {
            Operation::Shr32(a, b, c) => {
                cols.is_shl = F::ONE;
                cols.input_1 = b.transform(F::from_canonical_u8);
                cols.input_2 = c.transform(F::from_canonical_u8);
                cols.output = a.transform(F::from_canonical_u8);
            }
            Operation::Shl32(a, b, c) => {
                cols.is_shr = F::ONE;
                cols.input_1 = b.transform(F::from_canonical_u8);
                cols.input_2 = c.transform(F::from_canonical_u8);
                cols.output = a.transform(F::from_canonical_u8);
            }
        }

        row
    }
}

pub trait MachineWithShift32Chip: MachineWithCpuChip {
    fn shift_u32(&self) -> &Shift32Chip;
    fn shift_u32_mut(&mut self) -> &mut Shift32Chip;
}

instructions!(Shl32Instruction, Shr32Instruction);

impl<M> Instruction<M> for Shl32Instruction
where
    M: MachineWithShift32Chip + MachineWithRangeChip,
{
    const OPCODE: u32 = SHL32;

    fn execute(state: &mut M, ops: Operands<i32>) {
        let clk = state.cpu().clock;
        let mut imm: Option<Word<u8>> = None;
        let read_addr_1 = (state.cpu().fp as i32 + ops.b()) as u32;
        let write_addr = (state.cpu().fp as i32 + ops.a()) as u32;
        let b = state.mem_mut().read(clk, read_addr_1, true);
        let c = if ops.is_imm() == 1 {
            let c = (ops.c() as u32).into();
            imm = Some(c);
            c
        } else {
            let read_addr_2 = (state.cpu().fp as i32 + ops.c()) as u32;
            state.mem_mut().read(clk, read_addr_2, true)
        };

        let b_u32: u32 = b.into();
        let c_u32: u32 = c.into();
        let a = Word::from(b_u32 << c_u32);

        state.mem_mut().write(clk, write_addr, a, true);

        state
            .shift_u32_mut()
            .operations
            .push(Operation::Shl32(a, b, c));
        state
            .cpu_mut()
            .push_bus_op(imm, <Self as Instruction<M>>::OPCODE, ops);
    }
}

impl<M> Instruction<M> for Shr32Instruction
where
    M: MachineWithShift32Chip + MachineWithRangeChip,
{
    const OPCODE: u32 = SHR32;

    fn execute(state: &mut M, ops: Operands<i32>) {
        let clk = state.cpu().clock;
        let mut imm: Option<Word<u8>> = None;
        let read_addr_1 = (state.cpu().fp as i32 + ops.b()) as u32;
        let write_addr = (state.cpu().fp as i32 + ops.a()) as u32;
        let b = state.mem_mut().read(clk, read_addr_1, true);
        let c = if ops.is_imm() == 1 {
            let c = (ops.c() as u32).into();
            imm = Some(c);
            c
        } else {
            let read_addr_2 = (state.cpu().fp as i32 + ops.c()) as u32;
            state.mem_mut().read(clk, read_addr_2, true)
        };

        let b_u32: u32 = b.into();
        let c_u32: u32 = c.into();
        let a = Word::from(b_u32 >> c_u32);

        state.mem_mut().write(clk, write_addr, a, true);

        state
            .shift_u32_mut()
            .operations
            .push(Operation::Shl32(a, b, c));
        state
            .cpu_mut()
            .push_bus_op(imm, <Self as Instruction<M>>::OPCODE, ops);
    }
}
