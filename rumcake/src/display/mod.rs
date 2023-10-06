use embassy_futures::join::join_array;
use embassy_futures::select;
use embassy_time::{Duration, Ticker};
use embedded_graphics::prelude::DrawTarget;

use crate::hw::BATTERY_LEVEL;

#[cfg(feature = "usb")]
use crate::usb::USB_STATE;

pub mod drivers;

use self::drivers::DisplayDriver;

pub trait DisplayDevice {
    /// An FPS value of 0 will cause the display to only update when needed.
    /// Set this to a value higher than 0 if you are trying to display something with animations.
    const FPS: usize = 0;
}

macro_rules! on_update_default {
    () => {};
}

#[derive(Default)]
pub struct DisplayData {
    /// Current battery level
    pub battery_level: u8,
    pub usb_enabled: bool,
}

#[rumcake_macros::task]
pub async fn display_task<K: DisplayDevice>(display: impl DisplayDriver) {
    let mut display_state = DisplayData::default();

    let mut ticker = Ticker::every(if K::FPS == 0 {
        Duration::MAX // try to sleep forever
    } else {
        Duration::from_millis(1000 / K::FPS as u64)
    });

    let mut battery_level_subscriber = BATTERY_LEVEL.subscriber().unwrap();
    let mut usb_state_subscriber = USB_STATE.subscriber().unwrap();

    loop {
        let animation_fut = async {
            ticker.next().await;
        };

        let battery_level_fut = async {
            display_state.battery_level = battery_level_subscriber.next_message_pure().await;
        };

        let usb_state_fut = async {
            display_state.usb_enabled = usb_state_subscriber.next_message_pure().await;
        };

        display.on_update();
    }
}
