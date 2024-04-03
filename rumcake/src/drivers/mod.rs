//! An optional set of built-in drivers which implement rumcake's driver traits, so they can be used with rumcake tasks.

use embedded_io_async::{Read, Write};

#[cfg(feature = "is31fl3731")]
pub mod is31fl3731;

#[cfg(feature = "nrf-ble")]
pub mod nrf_ble;

#[cfg(feature = "ssd1306")]
pub mod ssd1306;

#[cfg(feature = "ws2812-bitbang")]
pub mod ws2812_bitbang;

/// Struct that allows you to use a serial driver (implementor of both [`embedded_io_async::Read`]
/// and [`embedded_io_async::Write`]) with rumcake. This can be used for split keyboards.
pub struct SerialSplitDriver<D: Write + Read> {
    /// A serial driver that implements the [`embedded_io_async::Read`] and
    /// [`embedded_io_async::Write`] traits.
    pub serial: D,
}

#[cfg(feature = "split-central")]
impl<D: Write + Read> crate::split::central::CentralDeviceDriver for SerialSplitDriver<D> {
    type DriverError = D::Error;

    async fn receive_message_from_peripherals(
        &mut self,
    ) -> Result<
        crate::split::MessageToCentral,
        crate::split::central::CentralDeviceError<Self::DriverError>,
    > {
        let mut buffer = [0; crate::split::MESSAGE_TO_CENTRAL_BUFFER_SIZE];
        self.serial.read_exact(&mut buffer).await?;
        postcard::from_bytes_cobs(&mut buffer)
            .map_err(crate::split::central::CentralDeviceError::DeserializationError)
    }

    async fn broadcast_message_to_peripherals(
        &mut self,
        message: crate::split::MessageToPeripheral,
    ) -> Result<(), crate::split::central::CentralDeviceError<Self::DriverError>> {
        let mut buffer = [0; crate::split::MESSAGE_TO_PERIPHERAL_BUFFER_SIZE];
        postcard::to_slice_cobs(&message, &mut buffer)
            .map_err(crate::split::central::CentralDeviceError::SerializationError)?;
        self.serial
            .write_all(&buffer)
            .await
            .map_err(crate::split::central::CentralDeviceError::DriverError)
    }
}

#[cfg(feature = "split-peripheral")]
impl<D: Write + Read> crate::split::peripheral::PeripheralDeviceDriver for SerialSplitDriver<D> {
    type DriverError = D::Error;

    async fn send_message_to_central(
        &mut self,
        event: crate::split::MessageToCentral,
    ) -> Result<(), crate::split::peripheral::PeripheralDeviceError<Self::DriverError>> {
        let mut buffer = [0; crate::split::MESSAGE_TO_CENTRAL_BUFFER_SIZE];
        postcard::to_slice_cobs(&event, &mut buffer)
            .map_err(crate::split::peripheral::PeripheralDeviceError::SerializationError)?;
        self.serial
            .write_all(&buffer)
            .await
            .map_err(crate::split::peripheral::PeripheralDeviceError::DriverError)
    }

    async fn receive_message_from_central(
        &mut self,
    ) -> Result<
        crate::split::MessageToPeripheral,
        crate::split::peripheral::PeripheralDeviceError<Self::DriverError>,
    > {
        let mut buffer = [0; crate::split::MESSAGE_TO_PERIPHERAL_BUFFER_SIZE];
        self.serial.read_exact(&mut buffer).await?;
        postcard::from_bytes_cobs(&mut buffer)
            .map_err(crate::split::peripheral::PeripheralDeviceError::DeserializationError)
    }
}
