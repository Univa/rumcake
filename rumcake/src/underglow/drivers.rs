use core::fmt::Debug;

use super::UnderglowDevice;

// Async version of SmartLedsWrite trait from `smart_leds`
pub trait UnderglowDriver<D: UnderglowDevice> {
    type DriverError: Debug;
    type Color;

    async fn write<T, I>(&mut self, iterator: T) -> Result<(), Self::DriverError>
    where
        T: Iterator<Item = I>,
        I: Into<Self::Color>;
}
