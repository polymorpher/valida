use core::borrow::{Borrow, BorrowMut};
use core::mem::{size_of, transmute};
use valida_derive::AlignedBorrow;
use valida_machine::Word;
use valida_util::indices_arr;

#[derive(AlignedBorrow, Default)]
pub struct Shift32Cols<T> {
    pub input_1: Word<T>,
    pub input_2: Word<T>,
    pub output: Word<T>,

    pub power_of_two: Word<T>,

    pub is_shl: T,
    pub is_shr: T,
}

pub const NUM_COLS: usize = size_of::<Shift32Cols<u8>>();
pub const COL_MAP: Shift32Cols<usize> = make_col_map();

const fn make_col_map() -> Shift32Cols<usize> {
    let indices_arr = indices_arr::<NUM_COLS>();
    unsafe { transmute::<[usize; NUM_COLS], Shift32Cols<usize>>(indices_arr) }
}
