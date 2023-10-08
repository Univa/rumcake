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

// TODO: fix horizontal layouts
macro_rules! on_update_default {
    ($display:ident, $direction:ident, $margin:literal) => {
        use embedded_graphics::prelude::Dimensions;
        use embedded_graphics::Drawable;
        use embedded_layout::prelude::Align;

        let bounding_box = $display.bounding_box();

        let contents =
            embedded_layout::prelude::Chain::new(embedded_text::TextBox::with_textbox_style(
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
                .push_str(&String::<3>::from(
                    crate::hw::BATTERY_LEVEL_STATE.get().await,
                ))
                .unwrap();
            string
        };

        #[cfg(any(feature = "bluetooth", feature = "split-driver-ble"))]
        let contents = contents.append(embedded_text::TextBox::with_textbox_style(
            &battery_level,
            bounding_box,
            DEFAULT_STYLE,
            DEFAULT_TEXTBOX_STYLE,
        ));

        // Mode
        #[cfg(all(feature = "usb", feature = "bluetooth"))]
        let contents = contents.append(embedded_text::TextBox::with_textbox_style(
            if crate::usb::USB_STATE.get().await {
                "MODE: USB"
            } else {
                "MODE: BT"
            },
            bounding_box,
            DEFAULT_STYLE,
            DEFAULT_TEXTBOX_STYLE,
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
