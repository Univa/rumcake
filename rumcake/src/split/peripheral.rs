//! Features for the "peripheral" device in a split keyboard setup.
//!
//! The "peripheral" device in a split keyboard setup defines a [`KeyboardMatrix`], and sends
//! matrix events to the central device (see [`MessageToCentral`](crate::split::MessageToCentral)).
//! A split keyboard setup could have more than one peripheral. If the split keyboard also uses
//! extra features, then all the peripherals should receive the related commands from the central
//! device (see [`MessageToPeripheral`]).

use core::fmt::Debug;

use defmt::{error, Debug2Format};
use embassy_futures::select::{select, Either};
use embassy_sync::channel::Channel;
use embassy_sync::pubsub::PubSubBehavior;
use embedded_io_async::ReadExactError;
use keyberon::layout::Event;
use postcard::Error;

use super::{MessageToCentral, MessageToPeripheral};
use crate::hw::platform::RawMutex;
use crate::keyboard::MATRIX_EVENTS;

// Trait that devices must implement to serve as a peripheral in a split keyboard setup.
pub trait PeripheralDevice {
    /// Get a reference to a channel that can receive matrix events from other tasks to be
    /// processed into keycodes.
    fn get_matrix_events_channel() -> &'static Channel<RawMutex, Event, 1> {
        static POLLED_EVENTS_CHANNEL: Channel<RawMutex, Event, 1> = Channel::new();

        &POLLED_EVENTS_CHANNEL
    }

    #[cfg(feature = "underglow")]
    type UnderglowDeviceType: crate::lighting::underglow::private::MaybeUnderglowDevice =
        crate::lighting::private::EmptyLightingDevice;

    #[cfg(feature = "simple-backlight")]
    type SimpleBacklightDeviceType: crate::lighting::simple_backlight::private::MaybeSimpleBacklightDevice =
        crate::lighting::private::EmptyLightingDevice;

    #[cfg(feature = "simple-backlight-matrix")]
    type SimpleBacklightMatrixDeviceType: crate::lighting::simple_backlight_matrix::private::MaybeSimpleBacklightMatrixDevice = crate::lighting::private::EmptyLightingDevice;

    #[cfg(feature = "rgb-backlight-matrix")]
    type RGBBacklightMatrixDeviceType: crate::lighting::rgb_backlight_matrix::private::MaybeRGBBacklightMatrixDevice = crate::lighting::private::EmptyLightingDevice;
}

pub(crate) mod private {
    use embassy_sync::channel::Channel;
    use keyberon::layout::Event;

    use crate::hw::platform::RawMutex;

    use super::PeripheralDevice;

    pub struct EmptyPeripheralDevice;
    impl MaybePeripheralDevice for EmptyPeripheralDevice {}

    pub trait MaybePeripheralDevice {
        fn get_matrix_events_channel() -> Option<&'static Channel<RawMutex, Event, 1>> {
            None
        }
    }

    impl<T: PeripheralDevice> MaybePeripheralDevice for T {
        fn get_matrix_events_channel() -> Option<&'static Channel<RawMutex, Event, 1>> {
            Some(T::get_matrix_events_channel())
        }
    }
}

/// A trait that a driver must implement to allow a peripheral device to send and receive messages from the central device.
pub trait PeripheralDeviceDriver {
    /// The type of error that the driver will return if it fails to receive or send a message.
    type DriverError: Debug;

    /// Send a [`MessageToCentral`] using the driver.
    async fn send_message_to_central(
        &mut self,
        event: MessageToCentral,
    ) -> Result<(), PeripheralDeviceError<Self::DriverError>>;

    /// Receive a message from the central device ([`MessageToPeripheral`]) using the driver.
    async fn receive_message_from_central(
        &mut self,
    ) -> Result<MessageToPeripheral, PeripheralDeviceError<Self::DriverError>>;
}

#[derive(Debug)]
/// Types of errors that can occur when a peripheral device sends and receives messages from a central device
pub enum PeripheralDeviceError<T> {
    /// Wrapper around an error provided by a driver implementation
    /// ([`PeripheralDeviceDriver::DriverError`]).
    DriverError(T),
    /// An error that can occur if the driver fails to deserialize the data from a central device
    /// into a [`MessageToPeripheral`].
    DeserializationError(Error),
    /// An error that can occur if the driver fails to serialize the data when sending a
    /// [`MessageToCentral`].
    SerializationError(Error),
    /// Reached an EOF unexpectedly when trying to receive data from a central device.
    UnexpectedEof,
}

impl<E> From<ReadExactError<E>> for PeripheralDeviceError<E> {
    fn from(value: ReadExactError<E>) -> Self {
        match value {
            ReadExactError::UnexpectedEof => PeripheralDeviceError::UnexpectedEof,
            ReadExactError::Other(e) => PeripheralDeviceError::DriverError(e),
        }
    }
}

// This task replaces the `layout_collect` task, which is usually used on non-split keyboards for sending events to the keyboard layout
pub async fn peripheral_task<K: PeripheralDevice>(_k: K, mut driver: impl PeripheralDeviceDriver) {
    let channel = K::get_matrix_events_channel();
    let matrix_event_publisher = MATRIX_EVENTS.immediate_publisher();

    loop {
        match select(
            driver.receive_message_from_central(),
            channel.receive(),
        )
        .await
        {
            Either::First(message) => match message {
                Ok(message) => match message {
                    #[cfg(feature = "simple-backlight")]
                    MessageToPeripheral::SimpleBacklight(command) => {
                        if let Some(channel) = <K::SimpleBacklightDeviceType as crate::lighting::simple_backlight::private::MaybeSimpleBacklightDevice>::get_command_channel() {
                            channel.send(command).await
                        }
                    }
                    #[cfg(feature = "simple-backlight-matrix")]
                    MessageToPeripheral::SimpleBacklightMatrix(command) => {
                        if let Some(channel) = <K::SimpleBacklightMatrixDeviceType as crate::lighting::simple_backlight_matrix::private::MaybeSimpleBacklightMatrixDevice>::get_command_channel() {
                            channel.send(command).await
                        }
                    }
                    #[cfg(feature = "rgb-backlight-matrix")]
                    MessageToPeripheral::RGBBacklightMatrix(command) => {
                        if let Some(channel) = <K::RGBBacklightMatrixDeviceType as crate::lighting::rgb_backlight_matrix::private::MaybeRGBBacklightMatrixDevice>::get_command_channel() {
                            channel.send(command).await
                        }
                    }
                    #[cfg(feature = "underglow")]
                    MessageToPeripheral::Underglow(command) => {
                        if let Some(channel) = <K::UnderglowDeviceType as crate::lighting::underglow::private::MaybeUnderglowDevice>::get_command_channel() {
                            channel.send(command).await
                        }
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
                matrix_event_publisher.publish_immediate(event);

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
