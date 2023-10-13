//! A trait that underglow drivers must implement.

use core::fmt::Debug;

use super::UnderglowDevice;

/// A trait that a driver must implement in order to work with the underglow task.
///
/// This is an async version of the [`smart_leds::SmartLedsWrite`] trait.
pub trait UnderglowDriver<D: UnderglowDevice> {
    /// The type of error that the driver will return if [`UnderglowDriver::write`] fails.
    type DriverError: Debug;

    /// The color used for frame buffers, to be consumed by the driver in [`UnderglowDriver::write`].
    type Color;

    /// Render out a frame buffer using the driver.
    async fn write<T, I>(&mut self, iterator: T) -> Result<(), Self::DriverError>
    where
        T: Iterator<Item = I>,
        I: Into<Self::Color>;
}
