//! A trait that underglow drivers must implement.

use core::fmt::Debug;

use smart_leds::RGB8;

use super::UnderglowDevice;

/// A trait that a driver must implement in order to work with the underglow task.
///
/// This is an async version of the [`smart_leds::SmartLedsWrite`] trait.
pub trait UnderglowDriver<D: UnderglowDevice> {
    /// The type of error that the driver will return if [`UnderglowDriver::write`] fails.
    type DriverWriteError: Debug;

    /// Render out a frame buffer using the driver.
    async fn write(
        &mut self,
        iterator: impl Iterator<Item = RGB8>,
    ) -> Result<(), Self::DriverWriteError>;

    /// The type of error that the driver will return if [`UnderglowDriver::turn_on`] fails.
    type DriverEnableError: Debug;

    /// Turn the LEDs on using the driver when the animator gets enabled.
    ///
    /// The animator's [`tick()`](super::animations::UnderglowAnimator::tick) method gets called
    /// directly after this, and subsequently [`UnderglowDriver::write`]. So, if your
    /// driver doesn't need do anything special to turn the LEDs on, you may simply return
    /// `Ok(())`.
    async fn turn_on(&mut self) -> Result<(), Self::DriverEnableError>;

    /// The type of error that the driver will return if [`UnderglowDriver::turn_off`] fails.
    type DriverDisableError: Debug;

    /// Turn the LEDs off using the driver when the animator is disabled.
    ///
    /// The animator's [`tick()`](super::animations::UnderglowAnimator::tick) method gets called
    /// directly after this. However, the tick method will not call
    /// [`UnderglowDriver::write`] due to the animator being disabled, so you will need to
    /// turn off the LEDs somehow. For example, you can write a brightness of 0 to all LEDs.
    async fn turn_off(&mut self) -> Result<(), Self::DriverDisableError>;
}
