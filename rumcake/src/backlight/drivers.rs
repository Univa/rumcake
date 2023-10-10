use core::fmt::Debug;

use super::{BacklightDevice, BacklightMatrixDevice};

pub trait SimpleBacklightDriver<K: BacklightDevice> {
    type DriverError: Debug;

    async fn write(&mut self, brightness: u8) -> Result<(), Self::DriverError>;
}

pub trait SimpleBacklightMatrixDriver<K: BacklightMatrixDevice>
where
    [(); K::MATRIX_COLS]:,
    [(); K::MATRIX_ROWS]:,
{
    type DriverError: Debug;

    async fn write(
        &mut self,
        buf: &[[u8; K::MATRIX_COLS]; K::MATRIX_ROWS],
    ) -> Result<(), Self::DriverError>;
}

pub trait RGBBacklightMatrixDriver<K: BacklightMatrixDevice>
where
    [(); K::MATRIX_COLS]:,
    [(); K::MATRIX_ROWS]:,
{
    type Color;
    type DriverError: Debug;

    async fn write(
        &mut self,
        buf: &[[Self::Color; K::MATRIX_COLS]; K::MATRIX_ROWS],
    ) -> Result<(), Self::DriverError>;
}
