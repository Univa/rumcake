use core::fmt::Debug;

use super::MessageToCentral;
use super::MessageToPeripheral;

pub trait CentralDeviceDriver {
    type DriverError: Debug;

    async fn receive_message_from_peripherals(
        &mut self,
    ) -> Result<MessageToCentral, CentralDeviceError<Self::DriverError>>;

    async fn broadcast_message_to_peripherals(
        &mut self,
        message: MessageToPeripheral,
    ) -> Result<(), CentralDeviceError<Self::DriverError>>;
}

#[derive(Debug)]
pub enum CentralDeviceError<E> {
    DriverError(E),
    DeserializationError([u8; 4]),
}

impl<E> From<E> for CentralDeviceError<E> {
    fn from(error: E) -> Self {
        CentralDeviceError::DriverError(error)
    }
}

pub trait PeripheralDeviceDriver {
    type DriverError: Debug;

    async fn send_message_to_central(
        &mut self,
        event: MessageToCentral,
    ) -> Result<(), PeripheralDeviceError<Self::DriverError>>;

    async fn receive_message_from_central(
        &mut self,
    ) -> Result<MessageToPeripheral, PeripheralDeviceError<Self::DriverError>>;
}

#[derive(Debug)]
pub enum PeripheralDeviceError<T> {
    DriverError(T),
}

impl<E> From<E> for PeripheralDeviceError<E> {
    fn from(error: E) -> Self {
        PeripheralDeviceError::DriverError(error)
    }
}
