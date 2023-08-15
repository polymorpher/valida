use super::columns::Shift32Cols;
use super::Shift32Chip;
use core::borrow::Borrow;
use valida_machine::MEMORY_CELL_BYTES;

use p3_air::{Air, AirBuilder};
use p3_field::AbstractField;
use p3_matrix::MatrixRows;

impl<F, AB> Air<AB> for Shift32Chip
where
    F: AbstractField,
    AB: AirBuilder<F = F>,
{
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let local: &Shift32Cols<AB::Var> = main.row(0).borrow();

        builder.assert_bool(local.is_shl);
        builder.assert_bool(local.is_shr);
        builder.assert_one(local.is_shl + local.is_shr);
    }
}
