use core::fmt::Debug;

use embedded_graphics::mono_font::ascii::FONT_5X8;
use embedded_graphics::mono_font::{MonoTextStyle, MonoTextStyleBuilder};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::Point;
use embedded_graphics::text::Text;
use embedded_graphics::Drawable;
use embedded_hal::blocking::i2c::Write;
use ssd1306::mode::BufferedGraphicsMode;
use ssd1306::prelude::{DisplayConfig, I2CInterface};
use ssd1306::rotation::DisplayRotation;
pub use ssd1306::size;
use ssd1306::size::{DisplaySize, DisplaySize128x32};
use ssd1306::{I2CDisplayInterface, Ssd1306};

use crate::display::DisplayDevice;

use super::DisplayDriver;

pub static DEFAULT_STYLE: MonoTextStyle<'_, BinaryColor> = MonoTextStyleBuilder::new()
    .font(&FONT_5X8)
    .text_color(BinaryColor::On)
    .build();

pub trait Ssd1306I2cDisplayDriver<S: DisplaySize = DisplaySize128x32>: DisplayDevice {
    const SIZE: S;
    const ROTATION: DisplayRotation = DisplayRotation::Rotate90;

    fn setup_i2c() -> impl Write<Error = impl Debug>;

    fn on_update(
        display: &mut Ssd1306<
            I2CInterface<impl Write<Error = impl Debug>>,
            S,
            BufferedGraphicsMode<S>,
        >,
    ) {
        Text::with_baseline(
            "test",
            Point::new(0, 16),
            DEFAULT_STYLE,
            embedded_graphics::text::Baseline::Top,
        )
        .draw(display)
        .unwrap();
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

impl<DI: Write<Error = impl Debug>, S: DisplaySize, K: Ssd1306I2cDisplayDriver<S>> DisplayDriver<K>
    for Ssd1306<I2CInterface<DI>, S, BufferedGraphicsMode<S>>
{
    fn on_update(&mut self) {
        K::on_update(self);
        self.flush().unwrap();
    }
}
