//! A set of traits that split keyboard drivers must implement, and error types that can be used by
//! driver implementations.

use core::fmt::Debug;

use super::MessageToCentral;
use super::MessageToPeripheral;

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
    /// Wrapper around an error provided by a driver implementation ([`CentralDeviceDriver::DriverError`]).
    DriverError(E),
    /// An error that can occur if the driver fails to deserialize the data from a peripheral into a [`MessageToCentral`].
    ///
    /// This variant will contain the data could not be deserialized.
    DeserializationError([u8; 4]),
}

impl<E> From<E> for CentralDeviceError<E> {
    fn from(error: E) -> Self {
        CentralDeviceError::DriverError(error)
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
    /// Wrapper around an error provided by a driver implementation ([`PeripheralDeviceDriver::DriverError`]).
    DriverError(T),
}

impl<E> From<E> for PeripheralDeviceError<E> {
    fn from(error: E) -> Self {
        PeripheralDeviceError::DriverError(error)
    }
}
