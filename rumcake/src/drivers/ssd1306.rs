pub mod driver {
    pub use ssd1306::*;
}

#[cfg(feature = "display")]
pub mod display {
    use core::fmt::Debug;

    use embedded_graphics::pixelcolor::BinaryColor;
    use embedded_graphics::prelude::DrawTarget;
    use embedded_hal::blocking::i2c::Write;
    use ssd1306::mode::BufferedGraphicsMode;
    use ssd1306::prelude::{DisplayConfig, I2CInterface};
    use ssd1306::rotation::DisplayRotation;
    use ssd1306::size::{DisplaySize, DisplaySize128x32};
    use ssd1306::{I2CDisplayInterface, Ssd1306};

    use crate::display::drivers::{on_update_default, DisplayDriver, Orientation};
    use crate::display::DisplayDevice;

    pub trait Ssd1306I2cDisplayDriver<S: DisplaySize = DisplaySize128x32>: DisplayDevice {
        const SIZE: S;
        const ROTATION: DisplayRotation = DisplayRotation::Rotate90;

        fn setup_i2c() -> impl Write<Error = impl Debug>;

        /// Update the SSD1306 screen. The frame buffer gets cleared before this function is called.
        /// After this function is called, the display will be flushed. So, an implementor simply
        /// needs to create the graphics to display on the screen, and does not need to clear the
        /// frame buffer or flush the data to the screen.
        async fn on_update(
            display: &mut Ssd1306<
                I2CInterface<impl Write<Error = impl Debug>>,
                S,
                BufferedGraphicsMode<S>,
            >,
        ) {
            match Self::ROTATION {
                DisplayRotation::Rotate0 | DisplayRotation::Rotate180 => {
                    on_update_default(display, Orientation::Horizontal, 8).await;
                }
                DisplayRotation::Rotate90 | DisplayRotation::Rotate270 => {
                    on_update_default(display, Orientation::Vertical, 12).await;
                }
            }
        }
    }

    pub async fn setup_display_driver<S: DisplaySize, K: Ssd1306I2cDisplayDriver<S>>(
        _k: K,
    ) -> impl DisplayDriver<K> {
        let mut display = Ssd1306::new(
            I2CDisplayInterface::new(K::setup_i2c()),
            K::SIZE,
            K::ROTATION,
        )
        .into_buffered_graphics_mode();
        display.init().unwrap();

        display
    }

    impl<DI: Write<Error = impl Debug>, S: DisplaySize, K: Ssd1306I2cDisplayDriver<S>>
        DisplayDriver<K> for Ssd1306<I2CInterface<DI>, S, BufferedGraphicsMode<S>>
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