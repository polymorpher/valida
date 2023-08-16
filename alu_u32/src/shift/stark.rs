use super::columns::Shift32Cols;
use super::Shift32Chip;
use core::borrow::Borrow;
use valida_machine::MEMORY_CELL_BYTES;

use p3_air::{Air, AirBuilder};
use p3_field::AbstractField;
use p3_matrix::MatrixRowSlices;

impl<F, AB> Air<AB> for Shift32Chip
where
    F: AbstractField,
    AB: AirBuilder<F = F>,
{
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let local: &Shift32Cols<AB::Var> = main.row_slice(0).borrow();

        let bit_base = [1, 2, 4, 8, 16, 32, 64, 128].map(AB::Expr::from_canonical_u32);
        let pow_base = [1 << 1, 1 << 2, 1 << 4, 1 << 8, 1 << 16].map(AB::Expr::from_canonical_u32);
        let byte_base = [1 << 24, 1 << 16, 1 << 8, 1].map(AB::Expr::from_canonical_u32);

        // Check that input byte decomposition is correct
        let byte_2: AB::Expr = local
            .bits_2
            .into_iter()
            .zip(bit_base.iter().cloned())
            .map(|(bit, base)| bit * base)
            .sum();
        builder.assert_eq(local.input_2[3], byte_2.clone());

        // Check that the power of two is correct (limited to 2^31)
        let temp_1 = (local.bits_2[0] * pow_base[0].clone())
            * (local.bits_2[1] * pow_base[1].clone())
            * (local.bits_2[2] * pow_base[2].clone());
        let temp_2 =
            (local.bits_2[3] * pow_base[3].clone()) * (local.bits_2[4] * pow_base[4].clone());
        builder.assert_eq(local.temp_1, temp_1);
        builder.assert_eq(local.temp_2, temp_2);

        let power_of_two = local.temp_1 * local.temp_2;
        let reduced_power_of_two: AB::Expr = local
            .power_of_two
            .into_iter()
            .enumerate()
            .map(|(i, x)| byte_base[i].clone() * x)
            .sum();
        builder.assert_eq(reduced_power_of_two, power_of_two);

        builder.assert_bool(local.is_shl);
        builder.assert_bool(local.is_shr);
        builder.assert_bool(local.is_shl + local.is_shr);
    }
}
