//! Features for the "peripheral" device in a split keyboard setup.
//!
//! The "peripheral" device in a split keyboard setup defines a [`KeyboardMatrix`], and sends
//! matrix events to the central device (see [`MessageToCentral`](crate::split::MessageToCentral)).
//! A split keyboard setup could have more than one peripheral. If the split keyboard also uses
//! extra features, then all the peripherals should receive the related commands from the central
//! device (see [`MessageToPeripheral`]).

use defmt::{error, Debug2Format};
use embassy_futures::select::{select, Either};
use embassy_sync::pubsub::PubSubBehavior;

use crate::keyboard::{KeyboardMatrix, MATRIX_EVENTS, POLLED_EVENTS_CHANNEL};
use crate::split::MessageToPeripheral;

use super::drivers::PeripheralDeviceDriver;

// This task replaces the `layout_collect` task, which is usually used on non-split keyboards for sending events to the keyboard layout
#[rumcake_macros::task]
pub async fn peripheral_task<K: KeyboardMatrix>(_k: K, mut driver: impl PeripheralDeviceDriver) {
    loop {
        match select(
            driver.receive_message_from_central(),
            POLLED_EVENTS_CHANNEL.receive(),
        )
        .await
        {
            Either::First(message) => match message {
                Ok(message) => match message {
                    #[cfg(any(
                        feature = "simple-backlight",
                        feature = "simple-backlight-matrix",
                        feature = "rgb-backlight-matrix"
                    ))]
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
