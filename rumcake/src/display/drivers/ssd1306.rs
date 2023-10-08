use core::fmt::Debug;

use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::{MonoTextStyle, MonoTextStyleBuilder};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::{Dimensions, DrawTarget};
use embedded_graphics::Drawable;
use embedded_hal::blocking::i2c::Write;
use embedded_layout::layout::linear::{FixedMargin, LinearLayout};
use embedded_layout::prelude::{horizontal, vertical, Align, Chain};
use embedded_text::alignment::HorizontalAlignment;
use embedded_text::style::{HeightMode, TextBoxStyle, TextBoxStyleBuilder};
use embedded_text::TextBox;
use heapless::String;
use ssd1306::mode::BufferedGraphicsMode;
use ssd1306::prelude::{DisplayConfig, I2CInterface};
use ssd1306::rotation::DisplayRotation;
use ssd1306::size::{DisplaySize, DisplaySize128x32};
use ssd1306::{I2CDisplayInterface, Ssd1306};

use crate::display::DisplayDevice;
use crate::hw::BATTERY_LEVEL_STATE;

#[cfg(feature = "usb")]
use crate::usb::USB_STATE;

use super::DisplayDriver;

pub mod driver {
    pub use ssd1306::*;
}

pub static DEFAULT_STYLE: MonoTextStyle<'_, BinaryColor> = MonoTextStyleBuilder::new()
    .font(&FONT_6X10)
    .text_color(BinaryColor::On)
    .build();

pub static DEFAULT_TEXTBOX_STYLE: TextBoxStyle = TextBoxStyleBuilder::new()
    .height_mode(HeightMode::FitToText)
    .alignment(HorizontalAlignment::Left)
    .build();

pub static DEFAULT_HEADER_STYLE: TextBoxStyle = TextBoxStyleBuilder::new()
    .height_mode(HeightMode::FitToText)
    .alignment(HorizontalAlignment::Center)
    .build();

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
        let bounding_box = display.bounding_box();

        let contents = Chain::new(TextBox::with_textbox_style(
            "INFO",
            bounding_box,
            DEFAULT_STYLE,
            DEFAULT_HEADER_STYLE,
        ));

        // Battery level
        #[cfg(any(feature = "bluetooth", feature = "split-driver-ble"))]
        let battery_level = {
            let mut string: String<8> = String::from("BAT: ");
            string
                .push_str(&String::<3>::from(BATTERY_LEVEL_STATE.get().await))
                .unwrap();
            string
        };

        #[cfg(any(feature = "bluetooth", feature = "split-driver-ble"))]
        let contents = contents.append(TextBox::with_textbox_style(
            &battery_level,
            bounding_box,
            DEFAULT_STYLE,
            DEFAULT_TEXTBOX_STYLE,
        ));

        // Mode
        #[cfg(all(feature = "usb", feature = "bluetooth"))]
        let contents = contents.append(TextBox::with_textbox_style(
            if USB_STATE.get().await {
                "MODE: USB"
            } else {
                "MODE: BT"
            },
            bounding_box,
            DEFAULT_STYLE,
            DEFAULT_TEXTBOX_STYLE,
        ));

        LinearLayout::vertical(contents)
            .with_spacing(FixedMargin(8))
            .align_to(&bounding_box, horizontal::Left, vertical::Top)
            .arrange()
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
