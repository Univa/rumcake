use embassy_futures::select::select_array;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Ticker};

pub mod drivers;

use self::drivers::DisplayDriver;

pub static USB_STATE_LISTENER: Signal<ThreadModeRawMutex, ()> = Signal::new();
pub static BATTERY_LEVEL_LISTENER: Signal<ThreadModeRawMutex, ()> = Signal::new();

pub trait DisplayDevice {
    /// An FPS value of 0 will make the display update only when needed.
    /// Set this to a value higher than 0 if you are trying to display something with animations.
    const FPS: usize = 0;

    /// How long the screen will stay on before it turns off due to inactivty.
    const TIMEOUT: usize = 30;
}

#[rumcake_macros::task]
pub async fn display_task<K: DisplayDevice>(mut display: impl DisplayDriver<K>) {
    let mut ticker = if K::FPS > 0 {
        Some(Ticker::every(Duration::from_millis(1000 / K::FPS as u64)))
    } else {
        None
    };

    loop {
        if let Some(ref mut ticker) = ticker {
            ticker.next().await;
        } else {
            select_array([USB_STATE_LISTENER.wait(), BATTERY_LEVEL_LISTENER.wait()]).await;
        }

        display.on_update();
    }
}
