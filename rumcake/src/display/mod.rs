use embassy_futures::select::{select, select_array, Either};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Ticker, Timer};

pub mod drivers;

use self::drivers::DisplayDriver;

pub static USB_STATE_LISTENER: Signal<ThreadModeRawMutex, ()> = Signal::new();
pub static BATTERY_LEVEL_LISTENER: Signal<ThreadModeRawMutex, ()> = Signal::new();

pub trait DisplayDevice {
    /// An FPS value of 0 will make the display update only when needed.
    /// Set this to a value higher than 0 if you are trying to display something with animations.
    const FPS: usize = 0;

    /// How long the screen will stay on before it turns off due to screen inactivty.
    /// If set to 0, the screen will always stay on.
    const TIMEOUT: usize = 30;
}

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

#[rumcake_macros::task]
pub async fn display_task<K: DisplayDevice>(mut display: impl DisplayDriver<K>) {
    let mut ticker = if K::FPS > 0 {
        Some(Ticker::every(Duration::from_millis(1000 / K::FPS as u64)))
    } else {
        None
    };

    // Tracks the state of the display so that we don't repeatedly send extra "turn_on" commands.
    display.turn_on().await;
    let mut display_on = true;

    // Render a frame after turning on
    display.on_update().await;

    loop {
        let update_fut = async {
            if let Some(ref mut ticker) = ticker {
                ticker.next().await;
                ((), 0)
            } else {
                let mut result =
                    select_array([USB_STATE_LISTENER.wait(), BATTERY_LEVEL_LISTENER.wait()]).await;
                result.1 += 1;
                result
            }
        };

        let timeout_timer = if K::TIMEOUT > 0 {
            Some(Timer::after(Duration::from_secs(K::TIMEOUT as u64)))
        } else {
            None
        };

        if let Some(timer) = timeout_timer {
            match select(update_fut, timer).await {
                Either::First(((), idx)) => {
                    match idx {
                        0 | 1 => {
                            // Turn the display on in the event of a tick, or change in USB state.
                            if !display_on {
                                display.turn_on().await;
                                display_on = true;
                            }
                        }
                        _ => {}
                    };

                    display.on_update().await;
                }
                Either::Second(_) => {
                    display.turn_off().await;
                    display_on = false;
                }
            };
        } else {
            update_fut.await;
            display.on_update().await;
        }
    }
}
