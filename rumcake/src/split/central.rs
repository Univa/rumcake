//! Features for the "central" device in a split keyboard setup.
//!
//! The "central" device in a split keyboard setup defines the [`KeyboardLayout`], communicates
//! with the host device (see [`crate::usb`] or [`crate::bluetooth`]), and receives matrix events
//! from other peripherals (see [`MessageToCentral`]). There should only be one central device. If
//! the split keyboard also uses extra features like backlighting or underglow, the central device
//! will also be responsible for sending their related commands to the peripherals (see
//! [`MessageToPeripheral`]).

use defmt::{error, Debug2Format};
use embassy_futures::select::{select, Either};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;

use crate::keyboard::{KeyboardLayout, POLLED_EVENTS_CHANNEL};
use crate::split::MessageToCentral;

use super::drivers::CentralDeviceDriver;
use super::MessageToPeripheral;

/// Channel for sending messages to peripherals.
///
/// Channel messages should be consumed by the central task, so user-level code should
/// **not** attempt to receive messages from the channel, otherwise commands may not be processed
/// appropriately. You should only send to this channel.
pub static MESSAGE_TO_PERIPHERALS: Channel<ThreadModeRawMutex, MessageToPeripheral, 4> =
    Channel::new();

#[rumcake_macros::task]
pub async fn central_task<K: KeyboardLayout>(_k: K, mut driver: impl CentralDeviceDriver) {
    loop {
        match select(
            driver.receive_message_from_peripherals(),
            MESSAGE_TO_PERIPHERALS.receive(),
        )
        .await
        {
            Either::First(message) => match message {
                Ok(event) => match event {
                    MessageToCentral::KeyPress(_, _) | MessageToCentral::KeyRelease(_, _) => {
                        POLLED_EVENTS_CHANNEL.send(event.try_into().unwrap()).await;
                    }
                },
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
