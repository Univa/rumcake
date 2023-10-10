#![no_std]
#![feature(stdsimd)]
#![feature(macro_metavar_expr)]
#![feature(generic_const_exprs)]
#![feature(async_fn_in_trait)]
#![feature(return_position_impl_trait_in_trait)]

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
    ($display:ident, "HORIZONTAL", $margin:literal) => {
        on_update_default!($display, horizontal, $margin, "text")
    };
    ($display:ident, "VERTICAL", $margin:literal) => {
        on_update_default!($display, vertical, $margin, "textbox")
    };
    ($display:ident, $direction:ident, $margin:literal, $text_type:tt) => {
        use embedded_graphics::prelude::Dimensions;
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
        #[cfg(any(feature = "bluetooth", feature = "split-driver-ble"))]
        let battery_level = {
            let mut string: String<8> = String::from("BAT: ");
            string
                .push_str(&String::<3>::from(
                    crate::hw::BATTERY_LEVEL_STATE.get().await,
                ))
                .unwrap();
            string
        };

        #[cfg(any(feature = "bluetooth", feature = "split-driver-ble"))]
        let contents = contents.append(crate::display::text_box!(
            bounding_box,
            $text_type,
            &battery_level
        ));

        // Mode
        #[cfg(all(feature = "usb", feature = "bluetooth"))]
        let contents = contents.append(crate::display::text_box!(
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

pub(crate) use on_update_default;
pub(crate) use text_box;

pub mod is31fl3731;
pub mod ssd1306;
pub mod ws2812_bitbang;
