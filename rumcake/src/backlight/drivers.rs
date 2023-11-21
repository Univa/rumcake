//! A set of traits that backlighting drivers must implement.

use core::fmt::Debug;

use smart_leds::RGB8;

use super::{BacklightDevice, BacklightMatrixDevice};

/// A trait that a driver must implement in order to support a simple (no matrix, one color) backlighting scheme.
pub trait SimpleBacklightDriver<K: BacklightDevice> {
    /// The type of error that the driver will return if [`SimpleBacklightDriver::write`] fails.
    type DriverWriteError: Debug;

    /// Render out a frame buffer using the driver.
    async fn write(&mut self, brightness: u8) -> Result<(), Self::DriverWriteError>;

    /// The type of error that the driver will return if [`SimpleBacklightDriver::turn_on`] fails.
    type DriverEnableError: Debug;

    /// Turn the LEDs on using the driver when the animator gets enabled.
    ///
    /// The animator's [`tick()`](super::animations::BacklightAnimator::tick) method gets called
    /// directly after this, and subsequently [`SimpleBacklightDriver::write`]. So, if your driver
    /// doesn't need do anything special to turn the LEDs on, you may simply return `Ok(())`.
    async fn turn_on(&mut self) -> Result<(), Self::DriverEnableError>;

    /// The type of error that the driver will return if [`SimpleBacklightDriver::turn_off`] fails.
    type DriverDisableError: Debug;

    /// Turn the LEDs off using the driver when the animator is disabled.
    ///
    /// The animator's [`tick()`](super::animations::BacklightAnimator::tick) method gets called
    /// directly after this. However, the tick method will not call
    /// [`SimpleBacklightDriver::write`] due to the animator being disabled, so you will need to
    /// turn off the LEDs somehow. For example, you can write a brightness of 0 to all LEDs.
    async fn turn_off(&mut self) -> Result<(), Self::DriverDisableError>;
}

/// A trait that a driver must implement in order to support a simple (no color) backlighting matrix scheme.
pub trait SimpleBacklightMatrixDriver<K: BacklightMatrixDevice> {
    /// The type of error that the driver will return if [`SimpleBacklightMatrixDriver::write`] fails.
    type DriverWriteError: Debug;

    /// Render out a frame buffer using the driver.
    async fn write(
        &mut self,
        buf: &[[u8; K::LIGHTING_COLS]; K::LIGHTING_ROWS],
    ) -> Result<(), Self::DriverWriteError>;

    /// The type of error that the driver will return if [`SimpleBacklightMatrixDriver::turn_on`] fails.
    type DriverEnableError: Debug;

    /// Turn the LEDs on using the driver when the animator gets enabled.
    ///
    /// The animator's [`tick()`](super::animations::BacklightAnimator::tick) method gets called
    /// directly after this, and subsequently [`SimpleBacklightMatrixDriver::write`]. So, if your
    /// driver doesn't need do anything special to turn the LEDs on, you may simply return
    /// `Ok(())`.
    async fn turn_on(&mut self) -> Result<(), Self::DriverEnableError>;

    /// The type of error that the driver will return if [`SimpleBacklightMatrixDriver::turn_off`] fails.
    type DriverDisableError: Debug;

    /// Turn the LEDs off using the driver when the animator is disabled.
    ///
    /// The animator's [`tick()`](super::animations::BacklightAnimator::tick) method gets called
    /// directly after this. However, the tick method will not call
    /// [`SimpleBacklightMatrixDriver::write`] due to the animator being disabled, so you will need to
    /// turn off the LEDs somehow. For example, you can write a brightness of 0 to all LEDs.
    async fn turn_off(&mut self) -> Result<(), Self::DriverDisableError>;
}

/// A trait that a driver must implement in order to support an RGB backlighting matrix scheme.
pub trait RGBBacklightMatrixDriver<K: BacklightMatrixDevice> {
    /// The type of error that the driver will return if [`RGBBacklightMatrixDriver::write`] fails.
    type DriverWriteError: Debug;

    /// Render out a frame buffer using the driver.
    async fn write(
        &mut self,
        buf: &[[RGB8; K::LIGHTING_COLS]; K::LIGHTING_ROWS],
    ) -> Result<(), Self::DriverWriteError>;

    /// The type of error that the driver will return if [`RGBBacklightMatrixDriver::turn_on`] fails.
    type DriverEnableError: Debug;

    /// Turn the LEDs on using the driver when the animator gets enabled.
    ///
    /// The animator's [`tick()`](super::animations::BacklightAnimator::tick) method gets called
    /// directly after this, and subsequently [`RGBBacklightMatrixDriver::write`]. So, if your
    /// driver doesn't need do anything special to turn the LEDs on, you may simply return
    /// `Ok(())`.
    async fn turn_on(&mut self) -> Result<(), Self::DriverEnableError>;

    /// The type of error that the driver will return if [`RGBBacklightMatrixDriver::turn_off`] fails.
    type DriverDisableError: Debug;

    /// Turn the LEDs off using the driver when the animator is disabled.
    ///
    /// The animator's [`tick()`](super::animations::BacklightAnimator::tick) method gets called
    /// directly after this. However, the tick method will not call
    /// [`RGBBacklightMatrixDriver::write`] due to the animator being disabled, so you will need to
    /// turn off the LEDs somehow. For example, you can write a brightness of 0 to all LEDs.
    async fn turn_off(&mut self) -> Result<(), Self::DriverDisableError>;
}
