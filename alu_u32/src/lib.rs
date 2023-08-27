#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use p3_field::AbstractField;

pub mod add;
pub mod bitwise;
pub mod div;
pub mod lt;
pub mod mul;
pub mod shift;
pub mod sub;

fn pad_to_power_of_two<const N: usize, F: AbstractField>(values: &mut Vec<F>) {
    let n_real_rows = values.len() / N;
    if n_real_rows > 0 {
        values.resize(n_real_rows.next_power_of_two() * N, F::ZERO);
    }
}
