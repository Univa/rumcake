//! Rumcake driver implementations for a custom WS2812 bitbang driver, which uses `nop`
//! instructions to simulate delay.
//!
//! This driver provides implementations for
//! [`UnderglowDriver`](`crate::underglow::drivers::UnderglowDriver`),
//! [`SimpleBacklightDriver`](`crate::backlight::drivers::SimpleBacklightDriver`),
//! [`SimpleBacklightMatrixDriver`](`crate::backlight::drivers::SimpleBacklightMatrixDriver`), and
//! [`RGBBacklightMatrixDriver`](`crate::backlight::drivers::RGBBacklightMatrixDriver`)
//!
//! To use this driver, pass the result of [`setup_driver`] to an underglow task, or backlight
//! task. If you want to use this driver as a backlight matrix, you will need to implement
//! [`WS2812BitbangBacklightDriver`](backlight::WS2812BitbangBacklightDriver).

use driver::Ws2812;
use embedded_hal::digital::v2::OutputPin;
pub use rumcake_macros::ws2812_bitbang_pin;

pub mod driver {
    use core::arch::arm::__nop as nop;

    use crate::hw::mcu::SYSCLK;
    use embassy_time::Duration;
    use embedded_hal::digital::v2::OutputPin;
    use smart_leds::RGB8;

    // TODO: move driver code below into its own crate?

    // in nanoseconds, taken from WS2812 datasheet
    const T0H: u64 = 350;
    const T0L: u64 = 900;

    const T1H: u64 = 900;
    const T1L: u64 = 350;

    // in microseconds
    const RES: u64 = 280;

    const fn gcd(a: u64, b: u64) -> u64 {
        if b == 0 {
            a
        } else {
            gcd(b, a % b)
        }
    }

    const NOP_FUDGE: f64 = 0.6;

    const TICK_CONV_FACTOR: f64 = (SYSCLK as u64 / gcd(SYSCLK as u64, 1_000_000_000)) as f64
        / (1_000_000_000 / gcd(SYSCLK as u64, 1_000_000_000)) as f64;

    pub struct Ws2812<P: OutputPin> {
        pin: P,
    }

    impl<P: OutputPin> Ws2812<P> {
        pub fn new(mut pin: P) -> Ws2812<P> {
            pin.set_low().ok();
            Self { pin }
        }

        #[inline(always)]
        pub fn write_byte(&mut self, mut data: u8) {
            for _ in 0..8 {
                if data & 0x80 == 0x80 {
                    self.pin.set_high().ok();
                    for _ in 0..(T1H as f64 * TICK_CONV_FACTOR * NOP_FUDGE) as i32 {
                        unsafe {
                            nop();
                        }
                    }
                    self.pin.set_low().ok();
                    for _ in 0..(T1L as f64 * TICK_CONV_FACTOR * NOP_FUDGE) as i32 {
                        unsafe {
                            nop();
                        }
                    }
                } else {
                    self.pin.set_high().ok();
                    for _ in 0..(T0H as f64 * TICK_CONV_FACTOR * NOP_FUDGE) as i32 {
                        unsafe {
                            nop();
                        }
                    }
                    self.pin.set_low().ok();
                    for _ in 0..(T0L as f64 * TICK_CONV_FACTOR * NOP_FUDGE) as i32 {
                        unsafe {
                            nop();
                        }
                    }
                };
                data <<= 1;
            }
        }

        pub fn write_colors(&mut self, colors: impl Iterator<Item = RGB8>) {
            for color in colors {
                self.write_byte(color.g);
                self.write_byte(color.r);
                self.write_byte(color.b);
            }

            // Reset time
            // Technically this isn't needed as long as the user sets a reasonable FPS value, but we'll keep it anyways.
            embassy_time::block_for(Duration::from_micros(RES));
        }
    }
}

/// Create an instance of the WS2812 bitbang driver with the provided output pin.
pub fn setup_driver(output_pin: impl OutputPin) -> Ws2812<impl OutputPin> {
    Ws2812::new(output_pin)
}

#[cfg(feature = "underglow")]
/// WS2812 underglow driver implementations
pub mod underglow {
    use embedded_hal::digital::v2::OutputPin;
    use smart_leds::gamma;
    use smart_leds::RGB8;

    use super::driver::Ws2812;
    use crate::underglow::drivers::UnderglowDriver;
    use crate::underglow::UnderglowDevice;

    impl<P: OutputPin, K: UnderglowDevice> UnderglowDriver<K> for Ws2812<P>
    where
        [(); K::NUM_LEDS]:,
    {
        type DriverWriteError = ();

        async fn write(
            &mut self,
            colors: impl Iterator<Item = RGB8>,
        ) -> Result<(), Self::DriverWriteError> {
            self.write_colors(gamma(colors));

            Ok(())
        }

        type DriverEnableError = ();

        async fn turn_on(&mut self) -> Result<(), Self::DriverEnableError> {
            // Don't need to do anything special, just let the next tick() get called.
            Ok(())
        }

        type DriverDisableError = ();

        async fn turn_off(&mut self) -> Result<(), Self::DriverDisableError> {
            self.write_colors([(0, 0, 0).into(); { K::NUM_LEDS }].iter().cloned());
            Ok(())
        }
    }
}

#[cfg(feature = "_backlight")]
/// WS2812 underglow driver implementations
pub mod backlight {
    use embedded_hal::digital::v2::OutputPin;
    use smart_leds::gamma;
    use smart_leds::RGB8;

    use super::driver::Ws2812;
    use crate::backlight::drivers::SimpleBacklightDriver;
    use crate::backlight::drivers::{RGBBacklightMatrixDriver, SimpleBacklightMatrixDriver};
    use crate::backlight::BacklightMatrixDevice;

    pub use rumcake_macros::ws2812_get_led_from_matrix_coordinates as get_led_from_matrix_coordinates;

    /// A trait that keyboards must implement to use the WS2812 driver for backlighting.
    pub trait WS2812BitbangBacklightDriver: BacklightMatrixDevice {
        /// Convert matrix coordinates in the form of (col, row) to a WS2812 LED index.
        ///
        /// It is recommended to use [`ws2812_get_led_from_matrix_coordinates`] to implement this function.
        fn get_led_from_matrix_coordinates(x: u8, y: u8) -> Option<u8>;
    }

    impl<P: OutputPin, K: WS2812BitbangBacklightDriver> SimpleBacklightDriver<K> for Ws2812<P>
    where
        [(); K::LIGHTING_ROWS * K::LIGHTING_COLS]:,
    {
        type DriverWriteError = ();

        async fn write(&mut self, brightness: u8) -> Result<(), Self::DriverWriteError> {
            let brightnesses = [(brightness, brightness, brightness).into(); {
                K::LIGHTING_ROWS * K::LIGHTING_COLS
            }];

            self.write_colors(gamma(brightnesses.iter().cloned()));

            Ok(())
        }

        type DriverEnableError = ();

        async fn turn_on(&mut self) -> Result<(), Self::DriverEnableError> {
            // Don't need to do anything special, just let the next tick() get called.
            Ok(())
        }

        type DriverDisableError = ();

        async fn turn_off(&mut self) -> Result<(), Self::DriverDisableError> {
            self.write_colors(
                [(0, 0, 0).into(); { K::LIGHTING_ROWS * K::LIGHTING_COLS }]
                    .iter()
                    .cloned(),
            );
            Ok(())
        }
    }

    impl<P: OutputPin, K: WS2812BitbangBacklightDriver> SimpleBacklightMatrixDriver<K> for Ws2812<P>
    where
        [(); K::LIGHTING_ROWS * K::LIGHTING_COLS]:,
    {
        type DriverWriteError = ();

        async fn write(
            &mut self,
            buf: &[[u8; K::LIGHTING_COLS]; K::LIGHTING_ROWS],
        ) -> Result<(), Self::DriverWriteError> {
            let mut brightnesses = [RGB8::default(); { K::LIGHTING_ROWS * K::LIGHTING_COLS }];

            for (row_num, row) in buf.iter().enumerate() {
                for (col_num, val) in row.iter().enumerate() {
                    if let Some(offset) =
                        K::get_led_from_matrix_coordinates(col_num as u8, row_num as u8)
                    {
                        brightnesses[offset as usize] = (*val, *val, *val).into();
                    }
                }
            }

            self.write_colors(gamma(brightnesses.iter().cloned()));

            Ok(())
        }

        type DriverEnableError = ();

        async fn turn_on(&mut self) -> Result<(), Self::DriverEnableError> {
            // Don't need to do anything special, just let the next tick() get called.
            Ok(())
        }

        type DriverDisableError = ();

        async fn turn_off(&mut self) -> Result<(), Self::DriverDisableError> {
            self.write_colors(
                [(0, 0, 0).into(); { K::LIGHTING_ROWS * K::LIGHTING_COLS }]
                    .iter()
                    .cloned(),
            );
            Ok(())
        }
    }

    impl<P: OutputPin, K: WS2812BitbangBacklightDriver> RGBBacklightMatrixDriver<K> for Ws2812<P>
    where
        [(); K::LIGHTING_ROWS * K::LIGHTING_COLS]:,
    {
        type DriverWriteError = ();

        async fn write(
            &mut self,
            buf: &[[RGB8; K::LIGHTING_COLS]; K::LIGHTING_ROWS],
        ) -> Result<(), Self::DriverWriteError> {
            let mut colors = [RGB8::default(); { K::LIGHTING_ROWS * K::LIGHTING_COLS }];

            for (row_num, row) in buf.iter().enumerate() {
                for (col_num, val) in row.iter().enumerate() {
                    if let Some(offset) =
                        K::get_led_from_matrix_coordinates(col_num as u8, row_num as u8)
                    {
                        colors[offset as usize] = *val;
                    }
                }
            }

            self.write_colors(gamma(colors.iter().cloned()));

            Ok(())
        }

        type DriverEnableError = ();

        async fn turn_on(&mut self) -> Result<(), Self::DriverEnableError> {
            // Don't need to do anything special, just let the next tick() get called.
            Ok(())
        }

        type DriverDisableError = ();

        async fn turn_off(&mut self) -> Result<(), Self::DriverDisableError> {
            self.write_colors(
                [(0, 0, 0).into(); { K::LIGHTING_ROWS * K::LIGHTING_COLS }]
                    .iter()
                    .cloned(),
            );
            Ok(())
        }
    }
}
