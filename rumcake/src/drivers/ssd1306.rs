//! Rumcake driver implementations for [jamwaffles's SSD1306 driver](`ssd1306`).
//!
//! This driver provides implementations for
//! [`DisplayDriver`](`crate::display::drivers::DisplayDriver`),
//!
//! To use this driver for the display feature, keyboards must implement
//! [`Ssd1306I2cDisplayDriver`](display::Ssd1306I2cDisplayDriver). The result of [`setup_driver`]
//! should be passed to a display task.

use core::fmt::Debug;

use embedded_hal::blocking::i2c::Write;
use ssd1306::mode::BufferedGraphicsMode;
use ssd1306::prelude::{DisplayConfig, I2CInterface};
use ssd1306::rotation::DisplayRotation;
use ssd1306::size::DisplaySize;
use ssd1306::{I2CDisplayInterface, Ssd1306};

pub use ssd1306 as driver;

/// Create an instance of the SSD1306 driver with the provided I2C peripheral, and default size and
/// rotation.
pub fn setup_driver<DI: Write<Error = impl Debug>, S: DisplaySize>(
    i2c: DI,
    size: S,
    rotation: DisplayRotation,
) -> Ssd1306<I2CInterface<DI>, S, BufferedGraphicsMode<S>> {
    let mut display =
        Ssd1306::new(I2CDisplayInterface::new(i2c), size, rotation).into_buffered_graphics_mode();
    display.init().unwrap();

    display
}

#[cfg(feature = "display")]
/// SSD1306 display driver implementations
pub mod display {
    use core::fmt::Debug;

    use embedded_graphics::pixelcolor::BinaryColor;
    use embedded_graphics::prelude::DrawTarget;
    use embedded_hal::blocking::i2c::Write;
    use ssd1306::mode::BufferedGraphicsMode;
    use ssd1306::prelude::I2CInterface;
    use ssd1306::rotation::DisplayRotation;
    use ssd1306::size::DisplaySize;
    use ssd1306::Ssd1306;

    use crate::display::drivers::{on_update_default, DisplayDriver, Orientation};
    use crate::display::DisplayDevice;

    /// A trait that keyboards must implement to use the SSD1306 driver for displaying information.
    pub trait Ssd1306I2cDisplayDriver: DisplayDevice {
        /// Update the SSD1306 screen. The frame buffer gets cleared before this function is called.
        /// After this function is called, the display will be flushed. So, an implementor simply
        /// needs to create the graphics to display on the screen, and does not need to clear the
        /// frame buffer or flush the data to the screen.
        async fn on_update<S: DisplaySize>(
            display: &mut Ssd1306<
                I2CInterface<impl Write<Error = impl Debug>>,
                S,
                BufferedGraphicsMode<S>,
            >,
        ) {
            match display.rotation() {
                DisplayRotation::Rotate0 | DisplayRotation::Rotate180 => {
                    on_update_default(display, Orientation::Horizontal, 8).await;
                }
                DisplayRotation::Rotate90 | DisplayRotation::Rotate270 => {
                    on_update_default(display, Orientation::Vertical, 12).await;
                }
            }
        }
    }

    impl<DI: Write<Error = impl Debug>, S: DisplaySize, K: Ssd1306I2cDisplayDriver> DisplayDriver<K>
        for Ssd1306<I2CInterface<DI>, S, BufferedGraphicsMode<S>>
    {
        async fn on_update(&mut self) {
            self.clear(BinaryColor::Off).unwrap();
            K::on_update(self).await;
            self.flush().unwrap();
        }

        async fn turn_off(&mut self) {
            self.set_display_on(false).unwrap();
        }

        async fn turn_on(&mut self) {
            self.set_display_on(true).unwrap();
        }
    }
}
