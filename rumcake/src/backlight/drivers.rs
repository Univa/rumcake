//! A set of traits that backlighting drivers must implement.

use core::fmt::Debug;

use super::{BacklightDevice, BacklightMatrixDevice};

/// A trait that a driver must implement in order to support a simple (no matrix, one color) backlighting scheme.
pub trait SimpleBacklightDriver<K: BacklightDevice> {
    /// The type of error that the driver will return if [`SimpleBacklightDriver::write`] fails.
    type DriverError: Debug;

    /// Render out a frame buffer using the driver.
    async fn write(&mut self, brightness: u8) -> Result<(), Self::DriverError>;
}

/// A trait that a driver must implement in order to support a simple (no color) backlighting matrix scheme.
pub trait SimpleBacklightMatrixDriver<K: BacklightMatrixDevice>
where
    [(); K::MATRIX_COLS]:,
    [(); K::MATRIX_ROWS]:,
{
    /// The type of error that the driver will return if [`SimpleBacklightMatrixDriver::write`] fails.
    type DriverError: Debug;

    /// Render out a frame buffer using the driver.
    async fn write(
        &mut self,
        buf: &[[u8; K::MATRIX_COLS]; K::MATRIX_ROWS],
    ) -> Result<(), Self::DriverError>;
}

/// A trait that a driver must implement in order to support an RGB backlighting matrix scheme.
pub trait RGBBacklightMatrixDriver<K: BacklightMatrixDevice>
where
    [(); K::MATRIX_COLS]:,
    [(); K::MATRIX_ROWS]:,
{
    /// The color used for frame buffers, to be consumed by the driver in [`RGBBacklightMatrixDriver::write`].
    type Color;

    /// The type of error that the driver will return if [`RGBBacklightMatrixDriver::write`] fails.
    type DriverError: Debug;

    /// Render out a frame buffer using the driver.
    async fn write(
        &mut self,
        buf: &[[Self::Color; K::MATRIX_COLS]; K::MATRIX_ROWS],
    ) -> Result<(), Self::DriverError>;
}
