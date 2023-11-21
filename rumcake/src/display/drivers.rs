//! A set of traits that display drivers must implement, and utilities that can be used by drivers.

use super::DisplayDevice;
use core::fmt::Debug;

use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::{MonoTextStyle, MonoTextStyleBuilder};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_text::alignment::HorizontalAlignment;
use embedded_text::style::{HeightMode, TextBoxStyle, TextBoxStyleBuilder};
use heapless::String;

/// Default style for text. The default style uses [`BinaryColor`], and [`FONT_6X10`].
pub static DEFAULT_STYLE: MonoTextStyle<'_, BinaryColor> = MonoTextStyleBuilder::new()
    .font(&FONT_6X10)
    .text_color(BinaryColor::On)
    .build();

/// Default style for text boxes. The default style uses [`HeightMode::FitToText`], and [`HorizontalAlignment::Left`]
pub static DEFAULT_TEXTBOX_STYLE: TextBoxStyle = TextBoxStyleBuilder::new()
    .height_mode(HeightMode::FitToText)
    .alignment(HorizontalAlignment::Left)
    .build();

macro_rules! text_box {
    ($box:expr, "textbox", $text:expr) => {
        embedded_text::TextBox::with_textbox_style(
            $text,
            $box,
            DEFAULT_STYLE,
            DEFAULT_TEXTBOX_STYLE,
        )
    };
    ($box:expr, "text", $text:expr) => {
        embedded_graphics::text::Text::new(
            $text,
            embedded_graphics::prelude::Point::zero(),
            DEFAULT_STYLE,
        )
    };
}

// TODO: fix default horizontal layout overflow if "FlowLayout" ever gets implemented: https://github.com/bugadani/embedded-layout/issues/8
macro_rules! on_update_default {
    ($display:ident, "HORIZONTAL", $margin:expr) => {
        on_update_default!($display, horizontal, $margin, "text")
    };
    ($display:ident, "VERTICAL", $margin:expr) => {
        on_update_default!($display, vertical, $margin, "textbox")
    };
    ($display:ident, $direction:ident, $margin:expr, $text_type:tt) => {
        use embedded_graphics::Drawable;
        use embedded_layout::prelude::Align;

        let bounding_box = $display.bounding_box();

        // Empty chain
        let contents = embedded_layout::prelude::Chain::new(embedded_graphics::text::Text::new(
            "",
            embedded_graphics::prelude::Point::zero(),
            DEFAULT_STYLE,
        ));

        // Battery level
        #[cfg(feature = "nrf-ble")]
        let battery_level = {
            let mut string: String<8> = String::from("BAT: ");
            string
                .push_str(&String::<3>::from(
                    crate::hw::BATTERY_LEVEL_STATE.get().await,
                ))
                .unwrap();
            string
        };

        #[cfg(feature = "nrf-ble")]
        let contents = contents.append(text_box!(bounding_box, $text_type, &battery_level));

        // Mode
        #[cfg(all(feature = "usb", feature = "bluetooth"))]
        let contents = contents.append(text_box!(
            bounding_box,
            $text_type,
            if crate::usb::USB_STATE.get().await {
                "MODE: USB"
            } else {
                "MODE: BT"
            }
        ));

        embedded_layout::layout::linear::LinearLayout::$direction(contents)
            .with_spacing(embedded_layout::layout::linear::FixedMargin($margin))
            .align_to(
                &bounding_box,
                embedded_layout::prelude::horizontal::Left,
                embedded_layout::prelude::vertical::Top,
            )
            .arrange()
            .draw($display)
            .unwrap();
    };
}

use embedded_graphics::prelude::DrawTarget;

/// Possible orientations for a display.
pub enum Orientation {
    /// Vertical/portrait orientation.
    Vertical,
    /// Horizontal/landscape orientation.
    Horizontal,
}

/// Default implementation for a display.
///
/// The default contents of the display will depend on what feature flags are
/// enabled. A list of possible data that may be shown includes:
/// - Battery level (BAT): `nrf-ble` must be enabled.
/// - Mode: `usb` and `bluetooth` enabled at the same time. See
/// [`rumcake::bluetooth::BluetoothCommand::ToggleOutput`]
pub async fn on_update_default(
    display: &mut impl DrawTarget<Color = BinaryColor, Error = impl Debug>,
    orientation: Orientation,
    margin: i32,
) {
    match orientation {
        Orientation::Vertical => {
            on_update_default!(display, "VERTICAL", margin);
        }
        Orientation::Horizontal => {
            on_update_default!(display, "HORIZONTAL", margin);
        }
    }
}

/// Trait that drivers must implement to work with the display task.
pub trait DisplayDriver<K: DisplayDevice> {
    /// Use the driver to update the display with new information.
    ///
    /// Called every time a data source updates, or every frame if [`DisplayDevice::FPS`] is non-zero.
    async fn on_update(&mut self);

    /// Use the driver to turn the display off.
    ///
    /// Called when the screen is being turned off. This usually occurs after [`DisplayDevice::TIMEOUT`] seconds.
    async fn turn_off(&mut self);

    /// Use the driver to turn the display on.
    ///
    /// Called when the screen is being turned back on after being turned off.
    async fn turn_on(&mut self);
}
