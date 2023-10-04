use crate::hw::drivers::ws2812_bitbang::Ws2812;
use crate::underglow::UnderglowDevice;
use embedded_hal::digital::v2::OutputPin;
use smart_leds::{gamma, RGB8};

use super::UnderglowDriver;

pub trait WS2812BitbangUnderglowDriver: UnderglowDevice {
    fn ws2812_pin() -> impl OutputPin;
}

pub async fn setup_underglow_driver<K: WS2812BitbangUnderglowDriver>() -> Ws2812<impl OutputPin> {
    Ws2812::new(K::ws2812_pin())
}

impl<P: OutputPin, K: UnderglowDevice> UnderglowDriver<K> for Ws2812<P> {
    type DriverError = ();
    type Color = RGB8;

    async fn write<T, I>(&mut self, colors: T) -> Result<(), Self::DriverError>
    where
        T: Iterator<Item = I>,
        I: Into<Self::Color>,
    {
        self.write_colors(gamma(colors.map(|c| c.into())));

        Ok(())
    }
}
