// #[cfg(feature = "underglow-driver-ws2812-bitbang")]
// pub mod ws2812_bitbang;
// #[cfg(feature = "underglow-driver-ws2812-bitbang")]
// pub use ws2812_bitbang::setup_underglow_driver;

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
