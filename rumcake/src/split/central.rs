use defmt::{error, Debug2Format};
use embassy_futures::select::{select, Either};
use embassy_sync::channel::Channel;
use embassy_sync::pubsub::PubSubBehavior;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use keyberon::layout::Layout;

use crate::keyboard::{KeyboardLayout, Keycode, MATRIX_EVENTS};
use crate::split::MessageToCentral;

use super::drivers::CentralDeviceDriver;
use super::MessageToPeripheral;

pub static MESSAGE_TO_PERIPHERALS: Channel<ThreadModeRawMutex, MessageToPeripheral, 4> =
    Channel::new();

// This task replaces the `layout_register` task, which is usually used on non-split keyboards for sending events to the keyboard layout
// Multiple instances of this task may run in order to send and receive messages from other peripherals
#[rumcake_macros::task]
pub async fn central_task<K: KeyboardLayout>(
    mut driver: impl CentralDeviceDriver,
    layout: &'static Mutex<
        ThreadModeRawMutex,
        Layout<{ K::LAYOUT_COLS }, { K::LAYOUT_ROWS }, { K::LAYERS }, Keycode>,
    >,
) {
    loop {
        match select(
            driver.receive_message_from_peripherals(),
            MESSAGE_TO_PERIPHERALS.receive(),
        )
        .await
        {
            Either::First(message) => match message {
                Ok(event) => {
                    let mut layout = layout.lock().await;
                    match event {
                        MessageToCentral::KeyPress(_, _) | MessageToCentral::KeyRelease(_, _) => {
                            let event = event.try_into();
                            layout.event(event.unwrap());
                            MATRIX_EVENTS.publish_immediate(event.unwrap());
                        }
                    }
                }
                Err(err) => {
                    error!(
                        "[SPLIT_CENTRAL] Error when attempting to receive from peripheral: {}",
                        Debug2Format(&err)
                    )
                }
            },
            Either::Second(message) => {
                if let Err(err) = driver.broadcast_message_to_peripherals(message).await {
                    error!(
                        "[SPLIT_CENTRAL] Error sending matrix events to peripheral: {}",
                        Debug2Format(&err)
                    )
                };
            }
        }
    }
}
