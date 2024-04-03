//! Rumcake driver implementations for a custom WS2812 bitbang driver, which uses `nop`
//! instructions to simulate delay.
//!
//! This driver provides implementations for
//! [`UnderglowDriver`](`crate::lighting::underglow::UnderglowDriver`),
//! [`SimpleBacklightDriver`](`crate::lighting::simple_backlight::SimpleBacklightDriver`),
//! [`SimpleBacklightMatrixDriver`](`crate::lighting::simple_backlight_matrix::SimpleBacklightMatrixDriver`),
//! and
//! [`RGBBacklightMatrixDriver`](`crate::lighting::rgb_backlight_matrix::RGBBacklightMatrixDriver`)
//!
//! To use this driver, pass the result of [`setup_driver`] to an underglow task, or backlight
//! task. If you want to use this driver as a backlight matrix, you will need to implement
//! [`WS2812BitbangBacklightDriver`](backlight::WS2812BitbangBacklightDriver).

use driver::Ws2812;
use embedded_hal::digital::v2::OutputPin;
use smart_leds::gamma;
use smart_leds::RGB8;

pub use rumcake_macros::{
    setup_ws2812_bitbang, ws2812_get_led_from_matrix_coordinates as get_led_from_matrix_coordinates,
};

pub mod driver {
    use core::arch::arm::__nop as nop;

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

    /// WS2812 driver. Const parameter C represents the clock speed of your chip, and F represents
    /// a fudge value. These const parameters determine together determine how many `nop`
    /// instructions are generated to simulate delay.
    pub struct Ws2812<const C: u32, const F: u8, P: OutputPin> {
        pin: P,
    }

    impl<const C: u32, const F: u8, P: OutputPin> Ws2812<C, F, P> {
        const NOP_FUDGE: f64 = 0.01 * F as f64;
        const TICK_CONV_FACTOR: f64 = (C as u64 / gcd(C as u64, 1_000_000_000)) as f64
            / (1_000_000_000 / gcd(C as u64, 1_000_000_000)) as f64;

        pub fn new(mut pin: P) -> Ws2812<C, F, P> {
            pin.set_low().ok();
            Self { pin }
        }

        #[inline(always)]
        pub fn write_byte(&mut self, mut data: u8) {
            for _ in 0..8 {
                if data & 0x80 == 0x80 {
                    self.pin.set_high().ok();
                    for _ in 0..(T1H as f64 * Self::TICK_CONV_FACTOR * Self::NOP_FUDGE) as i32 {
                        unsafe {
                            nop();
                        }
                    }
                    self.pin.set_low().ok();
                    for _ in 0..(T1L as f64 * Self::TICK_CONV_FACTOR * Self::NOP_FUDGE) as i32 {
                        unsafe {
                            nop();
                        }
                    }
                } else {
                    self.pin.set_high().ok();
                    for _ in 0..(T0H as f64 * Self::TICK_CONV_FACTOR * Self::NOP_FUDGE) as i32 {
                        unsafe {
                            nop();
                        }
                    }
                    self.pin.set_low().ok();
                    for _ in 0..(T0L as f64 * Self::TICK_CONV_FACTOR * Self::NOP_FUDGE) as i32 {
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
            for _ in 0..(RES as f64 * Self::TICK_CONV_FACTOR * Self::NOP_FUDGE) as i32 {
                unsafe {
                    nop();
                }
            }
        }
    }
}

/// Create an instance of the WS2812 bitbang driver with the provided output pin.
pub fn setup_driver<const C: u32, const F: u8>(
    output_pin: impl OutputPin,
) -> Ws2812<C, F, impl OutputPin> {
    Ws2812::new(output_pin)
}

/// A trait that keyboards must implement to use the WS2812 driver for simple backlighting.
pub trait WS2812BitbangSimpleBacklightDriver {
    /// Number of WS2812 LEDs powered by this driver.
    const NUM_LEDS: usize;
}

/// A trait that keyboards must implement to use the WS2812 driver for backlighting.
pub trait WS2812BitbangBacklightMatrixDriver {
    /// Convert matrix coordinates in the form of (col, row) to a WS2812 LED index.
    ///
    /// It is recommended to use [`ws2812_get_led_from_matrix_coordinates`] to implement this
    /// function.
    fn get_led_from_matrix_coordinates(x: u8, y: u8) -> Option<u8>;
}

#[cfg(feature = "underglow")]
impl<const C: u32, const F: u8, P: OutputPin, K: crate::lighting::underglow::UnderglowDevice>
    crate::lighting::underglow::UnderglowDriver<K> for Ws2812<C, F, P>
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

#[cfg(feature = "simple-backlight")]
impl<
        const C: u32,
        const F: u8,
        P: OutputPin,
        K: WS2812BitbangSimpleBacklightDriver
            + crate::lighting::simple_backlight::SimpleBacklightDevice,
    > crate::lighting::simple_backlight::SimpleBacklightDriver<K> for Ws2812<C, F, P>
where
    [(); K::NUM_LEDS]:,
{
    type DriverWriteError = ();

    async fn write(&mut self, brightness: u8) -> Result<(), Self::DriverWriteError> {
        let brightnesses = [(brightness, brightness, brightness).into(); K::NUM_LEDS];

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
        self.write_colors([(0, 0, 0).into(); K::NUM_LEDS].iter().cloned());
        Ok(())
    }
}

#[cfg(feature = "simple-backlight-matrix")]
impl<
        const C: u32,
        const F: u8,
        P: OutputPin,
        K: WS2812BitbangBacklightMatrixDriver
            + crate::lighting::simple_backlight_matrix::SimpleBacklightMatrixDevice,
    > crate::lighting::simple_backlight_matrix::SimpleBacklightMatrixDriver<K> for Ws2812<C, F, P>
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

#[cfg(feature = "rgb-backlight-matrix")]
impl<
        const C: u32,
        const F: u8,
        P: OutputPin,
        K: WS2812BitbangBacklightMatrixDriver
            + crate::lighting::rgb_backlight_matrix::RGBBacklightMatrixDevice,
    > crate::lighting::rgb_backlight_matrix::RGBBacklightMatrixDriver<K> for Ws2812<C, F, P>
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
