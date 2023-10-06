use defmt::{error, Debug2Format};
use embassy_futures::select::{select, Either};
use embassy_sync::pubsub::PubSubBehavior;

use crate::keyboard::{KeyboardMatrix, MATRIX_EVENTS, POLLED_EVENTS_CHANNEL};
use crate::split::MessageToPeripheral;

use super::drivers::PeripheralDeviceDriver;

// This task replaces the `layout_register` task, which is usually used on non-split keyboards for sending events to the keyboard layout
#[rumcake_macros::task]
pub async fn peripheral_task<K: KeyboardMatrix>(mut driver: impl PeripheralDeviceDriver) {
    loop {
        match select(
            driver.receive_message_from_central(),
            POLLED_EVENTS_CHANNEL.receive(),
        )
        .await
        {
            Either::First(message) => match message {
                Ok(message) => match message {
                    #[cfg(feature = "backlight")]
                    MessageToPeripheral::Backlight(command) => {
                        crate::backlight::BACKLIGHT_COMMAND_CHANNEL
                            .send(command)
                            .await
                    }
                    #[cfg(feature = "underglow")]
                    MessageToPeripheral::Underglow(command) => {
                        crate::underglow::UNDERGLOW_COMMAND_CHANNEL
                            .send(command)
                            .await
                    }
                    #[allow(unreachable_patterns)]
                    _ => {}
                },
                Err(err) => {
                    error!(
                        "[SPLIT_PERIPHERAL] Error when attempting to receive from central: {}",
                        Debug2Format(&err)
                    )
                }
            },
            Either::Second(event) => {
                MATRIX_EVENTS.publish_immediate(event);

                if let Err(err) = driver.send_message_to_central(event.into()).await {
                    error!(
                        "[SPLIT_PERIPHERAL] Error sending matrix events to central: {}",
                        Debug2Format(&err)
                    )
                };
            }
        }
    }
}
