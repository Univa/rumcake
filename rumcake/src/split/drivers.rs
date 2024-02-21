//! A set of traits that split keyboard drivers must implement, and error types that can be used by
//! driver implementations.

use core::fmt::Debug;

use embedded_io_async::ReadExactError;
use embedded_io_async::{Read, Write};
use postcard::Error;

use super::{MessageToCentral, MESSAGE_TO_CENTRAL_BUFFER_SIZE};
use super::{MessageToPeripheral, MESSAGE_TO_PERIPHERAL_BUFFER_SIZE};

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

/// Struct that allows you to use a serial driver (implementor of both [`embedded_io_async::Read`]
/// and [`embedded_io_async::Write`]) with rumcake's split keyboard tasks.
pub struct SerialSplitDriver<D: Write + Read> {
    /// A serial driver that implements the [`embedded_io_async::Read`] and
    /// [`embedded_io_async::Write`] traits.
    pub serial: D,
}

impl<D: Write + Read> CentralDeviceDriver for SerialSplitDriver<D> {
    type DriverError = D::Error;

    async fn receive_message_from_peripherals(
        &mut self,
    ) -> Result<MessageToCentral, CentralDeviceError<Self::DriverError>> {
        let mut buffer = [0; MESSAGE_TO_CENTRAL_BUFFER_SIZE];
        self.serial.read_exact(&mut buffer).await?;
        postcard::from_bytes_cobs(&mut buffer).map_err(CentralDeviceError::DeserializationError)
    }

    async fn broadcast_message_to_peripherals(
        &mut self,
        message: MessageToPeripheral,
    ) -> Result<(), CentralDeviceError<Self::DriverError>> {
        let mut buffer = [0; MESSAGE_TO_PERIPHERAL_BUFFER_SIZE];
        postcard::to_slice_cobs(&message, &mut buffer)
            .map_err(CentralDeviceError::SerializationError)?;
        self.serial
            .write_all(&buffer)
            .await
            .map_err(CentralDeviceError::DriverError)
    }
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

impl<D: Write + Read> PeripheralDeviceDriver for SerialSplitDriver<D> {
    type DriverError = D::Error;

    async fn send_message_to_central(
        &mut self,
        event: MessageToCentral,
    ) -> Result<(), PeripheralDeviceError<Self::DriverError>> {
        let mut buffer = [0; MESSAGE_TO_CENTRAL_BUFFER_SIZE];
        postcard::to_slice_cobs(&event, &mut buffer)
            .map_err(PeripheralDeviceError::SerializationError)?;
        self.serial
            .write_all(&buffer)
            .await
            .map_err(PeripheralDeviceError::DriverError)
    }

    async fn receive_message_from_central(
        &mut self,
    ) -> Result<MessageToPeripheral, PeripheralDeviceError<Self::DriverError>> {
        let mut buffer = [0; MESSAGE_TO_PERIPHERAL_BUFFER_SIZE];
        self.serial.read_exact(&mut buffer).await?;
        postcard::from_bytes_cobs(&mut buffer).map_err(PeripheralDeviceError::DeserializationError)
    }
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
