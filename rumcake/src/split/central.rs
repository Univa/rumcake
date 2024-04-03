//! Features for the "central" device in a split keyboard setup.
//!
//! The "central" device in a split keyboard setup defines the [`KeyboardLayout`], communicates
//! with the host device (see [`crate::usb`] or [`crate::bluetooth`]), and receives matrix events
//! from other peripherals (see [`MessageToCentral`]). There should only be one central device. If
//! the split keyboard also uses extra features like backlighting or underglow, the central device
//! will also be responsible for sending their related commands to the peripherals (see
//! [`MessageToPeripheral`]).

use core::fmt::Debug;

use defmt::{error, Debug2Format};
use embassy_futures::select::{select, Either};
use embassy_sync::channel::Channel;
use embedded_io_async::ReadExactError;
use postcard::Error;

use super::{MessageToCentral, MessageToPeripheral};
use crate::hw::platform::RawMutex;
use crate::keyboard::KeyboardLayout;

pub trait CentralDevice {
    /// The layout to send matrix events (which were received by peripherals) to.
    type Layout: KeyboardLayout;

    /// Get a reference to a channel that can receive messages from other tasks to be sent to
    /// peripherals.
    fn get_message_to_peripheral_channel() -> &'static Channel<RawMutex, MessageToPeripheral, 4> {
        static MESSAGE_TO_PERIPHERALS: Channel<RawMutex, MessageToPeripheral, 4> = Channel::new();

        &MESSAGE_TO_PERIPHERALS
    }
}

pub(crate) mod private {
    use embassy_sync::channel::Channel;

    use crate::hw::platform::RawMutex;
    use crate::split::MessageToPeripheral;

    use super::CentralDevice;

    pub struct EmptyCentralDevice;
    impl MaybeCentralDevice for EmptyCentralDevice {}

    pub trait MaybeCentralDevice {
        #[inline(always)]
        fn get_message_to_peripheral_channel(
        ) -> Option<&'static Channel<RawMutex, MessageToPeripheral, 4>> {
            None
        }
    }

    impl<T: CentralDevice> MaybeCentralDevice for T {
        #[inline(always)]
        fn get_message_to_peripheral_channel(
        ) -> Option<&'static Channel<RawMutex, MessageToPeripheral, 4>> {
            Some(T::get_message_to_peripheral_channel())
        }
    }
}

/// A trait that a driver must implement to allow a central device to send and receive messages from peripherals.
pub trait CentralDeviceDriver {
    /// The type of error that the driver will return if it fails to receive or send a message.
    type DriverError: Debug;

    /// Receive a message from a peripheral device ([`MessageToCentral`]).
    async fn receive_message_from_peripherals(
        &mut self,
    ) -> Result<MessageToCentral, CentralDeviceError<Self::DriverError>>;

    /// Send a [`MessageToPeripheral`] to all connected peripherals using the driver.
    async fn broadcast_message_to_peripherals(
        &mut self,
        message: MessageToPeripheral,
    ) -> Result<(), CentralDeviceError<Self::DriverError>>;
}

#[derive(Debug)]
/// Types of errors that can occur when a central device sends and receives messages from peripherals
pub enum CentralDeviceError<E> {
    /// Wrapper around an error provided by a driver implementation
    /// ([`CentralDeviceDriver::DriverError`]).
    DriverError(E),
    /// An error that can occur if the driver fails to deserialize the data from a peripheral into
    /// a [`MessageToCentral`].
    DeserializationError(Error),
    /// An error that can occur if the driver fails to serialize the data when sending a
    /// [`MessageToPeripheral`].
    SerializationError(Error),
    /// Reached an EOF unexpectedly when trying to receive data from a peripheral.
    UnexpectedEof,
}

impl<E> From<ReadExactError<E>> for CentralDeviceError<E> {
    fn from(value: ReadExactError<E>) -> Self {
        match value {
            ReadExactError::UnexpectedEof => CentralDeviceError::UnexpectedEof,
            ReadExactError::Other(e) => CentralDeviceError::DriverError(e),
        }
    }
}

#[rumcake_macros::task]
pub async fn central_task<K: CentralDevice>(_k: K, mut driver: impl CentralDeviceDriver) {
    let message_to_peripherals_channel = K::get_message_to_peripheral_channel();
    let matrix_events_channel = K::Layout::get_matrix_events_channel();

    loop {
        match select(
            driver.receive_message_from_peripherals(),
            message_to_peripherals_channel.receive(),
        )
        .await
        {
            Either::First(message) => match message {
                Ok(event) => match event {
                    MessageToCentral::KeyPress(_, _) | MessageToCentral::KeyRelease(_, _) => {
                        matrix_events_channel.send(event.try_into().unwrap()).await;
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
