//! Display feature.
//!
//! To use the display feature, keyboards must implement [`DisplayDevice`], along
//! with the trait corresponding to the chosen driver (which should implement
//! [`drivers::DisplayDriver`]).

use embassy_futures::select::{select, select_array, Either};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Ticker, Timer};

pub mod drivers;

use self::drivers::DisplayDriver;

pub(crate) static USB_STATE_LISTENER: Signal<ThreadModeRawMutex, ()> = Signal::new();
pub(crate) static BATTERY_LEVEL_LISTENER: Signal<ThreadModeRawMutex, ()> = Signal::new();

/// A trait that keyboards must implement to use a display.
pub trait DisplayDevice {
    /// An FPS value of 0 will make the display update only when needed.
    ///
    /// Set this to a value higher than 0 if you are trying to display something with animations.
    const FPS: usize = 0;

    /// How long the screen will stay on before it turns off due to screen inactivty.
    ///
    /// If set to 0, the screen will always stay on.
    const TIMEOUT: usize = 30;
}

#[rumcake_macros::task]
pub async fn display_task<K: DisplayDevice>(_k: K, mut display: impl DisplayDriver<K>) {
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
